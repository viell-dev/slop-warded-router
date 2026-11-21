//! Selection rules layered over existing grammar parsers, not a field parser.
//!
//! parse-net's own media and language selection differs from Warded's documented rules in
//! the cases noted below, so only its parsing and range matching are reused here. Its
//! language lookup refuses only an offer equal to a `q=0` range and counts `*` only as the
//! last range in priority; its basic filtering breaks equal weights by range length.
//! Encoding selection is parse-net's, except for the rank of an unlisted `identity`.

use super::{Choice, LanguageMatching};
use parse_net::{
    Limits, media,
    negotiate::{MalformedPolicy, Negotiator, Outcome, parse_weighted},
};

// RFC 9110 section 12.5.3 makes an unlisted `identity` acceptable without giving it a weight,
// and asks for no coding when none of the available codings is listed as acceptable. parse-net
// 0.10.0 instead weights it as `q=1`, so it outranks lower listed codings and wins offer-order
// ties. When parse-net chooses an `identity` that no `identity` or `*` member weights, choose
// again among the other offers and keep `identity` only if none of them is acceptable. Every
// other answer, including refusals, the empty field and explicit or `*` weights, is parse-net's.
pub(super) fn encoding(value: Option<&[u8]>, offers: &[&str], limits: Limits) -> Choice {
    let choose = |offers: &[&str]| match Negotiator::new()
        .limits(limits)
        .on_malformed(MalformedPolicy::Refuse)
        .encoding(value, offers)
    {
        Outcome::Chosen { index, .. } => Ok(Some(index)),
        Outcome::NoneAcceptable => Ok(None),
        Outcome::Refused(error) => Err(error.to_string()),
        _ => Err("unexpected encoding negotiation result under refusal policy".into()),
    };
    let chosen = choose(offers)?;
    let identity = chosen.filter(|&index| offers[index].eq_ignore_ascii_case("identity"));
    // An absent field states no preference, so its first-offer choice stands.
    let (Some(value), Some(identity)) = (value, identity) else {
        return Ok(chosen);
    };
    // parse-net has just validated this field with the same limits.
    let members = parse_weighted(value, &limits).map_err(|e| e.to_string())?;
    if members
        .iter()
        .any(|member| member.value == b"*" || member.value.eq_ignore_ascii_case(b"identity"))
    {
        return Ok(chosen);
    }
    let others = [&offers[..identity], &offers[identity + 1..]].concat();
    let listed = choose(&others)?.map(|index| index + usize::from(index >= identity));
    Ok(listed.or(chosen))
}

/// Wildcard/type specificity, then the number of parameters other than `q`.
///
/// `MediaRange::specificity` has the same order but stops counting at 63 parameters, while a
/// member may carry 64: ranges of 63 and 64 parameters would tie and the earlier would apply.
fn specificity(range: &media::MediaRange<'_>) -> (u8, usize) {
    let kind = u8::from(range.type_() != b"*") + u8::from(range.subtype() != b"*");
    (kind, range.params().len())
}

pub(super) fn media(value: Option<&[u8]>, offers: &[&str], limits: Limits) -> Choice {
    let Some(value) = value else {
        return Ok((!offers.is_empty()).then_some(0));
    };
    let list = parse_net::list::parse(value, &limits).map_err(|e| e.to_string())?;
    // parse-net reads parameters on either side of `q` and compares them with an offer's
    // (names and charset values without case, other values exactly after unquoting).
    let ranges = list
        .iter()
        .map(|member| media::parse_range(member, &limits).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut chosen = None;
    for (index, offer) in offers.iter().enumerate() {
        let offer = media::parse(offer.as_bytes(), &Limits::DEFAULT).map_err(|e| e.to_string())?;
        let mut best = None;
        for range in &ranges {
            let specificity = specificity(range);
            if range.matches(&offer) && best.is_none_or(|(rank, _)| specificity > rank) {
                best = Some((specificity, range.weight().thousandths()));
            }
        }
        if let Some((_, quality)) = best
            && quality > 0
            && chosen.is_none_or(|(_, previous)| quality > previous)
        {
            chosen = Some((index, quality));
        }
    }
    Ok(chosen.map(|(index, _)| index))
}

fn basic_matches(range: &[u8], offer: &[u8]) -> bool {
    range == b"*"
        || range.eq_ignore_ascii_case(offer)
        || (offer.len() > range.len()
            && offer[range.len()] == b'-'
            && offer[..range.len()].eq_ignore_ascii_case(range))
}

// RFC 4647 section 3.4: a single-letter or digit subtag is removed together with
// its closest trailing subtag, so no truncation ends in one. Consecutive
// singletons (`x-a-b-cc`) are all removed in the same step.
fn truncate(range: &[u8]) -> Option<&[u8]> {
    let cut = range.iter().rposition(|&b| b == b'-')?;
    let mut shorter = &range[..cut];
    while let Some(dash) = shorter.iter().rposition(|&b| b == b'-')
        && shorter.len() - dash == 2
    {
        shorter = &shorter[..dash];
    }
    (shorter.len() > 1).then_some(shorter)
}

// Rank is used only for zero-quality exclusions. Positive lookup preferences
// are processed by client priority, never overwritten by lower-priority ranges.
//
// A truncation equals the offer exactly when the offer is a proper prefix at a subtag
// boundary and ends in a multi-character subtag. Checking the prefix and its
// last subtag takes linear time in the offer length.
fn lookup_rank(range: &[u8], offer: &[u8]) -> Option<usize> {
    if range == b"*" {
        return Some(0);
    }
    if range.eq_ignore_ascii_case(offer) {
        return Some(usize::MAX);
    }
    let last_subtag = offer
        .iter()
        .rposition(|&b| b == b'-')
        .map_or(offer.len(), |dash| offer.len() - dash - 1);
    (last_subtag > 1
        && range.len() > offer.len()
        && range[offer.len()] == b'-'
        && range[..offer.len()].eq_ignore_ascii_case(offer))
    .then_some(offer.len())
}

pub(super) fn language(
    value: Option<&[u8]>,
    offers: &[&str],
    mode: LanguageMatching,
    limits: Limits,
) -> Choice {
    let Some(value) = value else {
        return Ok((!offers.is_empty()).then_some(0));
    };
    // Reuse the dependency's strict language-range/weight validation. Passing
    // no offers exercises validation without relying on its selection scorer.
    if let Outcome::Refused(error) = Negotiator::new()
        .limits(limits)
        .on_malformed(MalformedPolicy::Refuse)
        .language(Some(value), &[])
    {
        return Err(error.to_string());
    }
    let parsed = parse_weighted(value, &limits).map_err(|e| e.to_string())?;
    let mut ranges: Vec<_> = parsed.iter().collect();
    if mode == LanguageMatching::BasicFilter {
        let mut chosen = None;
        for (index, offer) in offers.iter().enumerate() {
            let mut best = None;
            for range in &ranges {
                let rank = if range.value == b"*" {
                    0
                } else {
                    range.value.len()
                };
                if basic_matches(range.value, offer.as_bytes())
                    && best.is_none_or(|(r, _)| rank > r)
                {
                    best = Some((rank, range.weight.thousandths()));
                }
            }
            if let Some((_, quality)) = best
                && quality > 0
                && chosen.is_none_or(|(_, q)| quality > q)
            {
                chosen = Some((index, quality));
            }
        }
        return Ok(chosen.map(|(index, _)| index));
    }
    ranges.sort_by_key(|range| std::cmp::Reverse(range.weight.thousandths()));
    let excluded = |offer: &[u8], positive_rank| {
        ranges.iter().any(|range| {
            range.weight.thousandths() == 0
                && lookup_rank(range.value, offer).is_some_and(|rank| rank >= positive_rank)
        })
    };
    for range in ranges
        .iter()
        .filter(|range| range.value != b"*" && range.weight.thousandths() > 0)
    {
        let mut current = Some(range.value);
        while let Some(candidate) = current {
            let rank = if candidate == range.value {
                usize::MAX
            } else {
                candidate.len()
            };
            if let Some(index) = offers.iter().position(|offer| {
                candidate.eq_ignore_ascii_case(offer.as_bytes())
                    && !excluded(offer.as_bytes(), rank)
            }) {
                return Ok(Some(index));
            }
            current = truncate(candidate);
        }
    }
    if ranges
        .iter()
        .any(|range| range.value == b"*" && range.weight.thousandths() > 0)
    {
        return Ok(offers
            .iter()
            .position(|offer| !excluded(offer.as_bytes(), 0)));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{lookup_rank, truncate};

    #[test]
    fn lookup_exclusion_rank_agrees_with_iterative_truncation() {
        let check = |range: &[u8], offer: &[u8]| {
            let expected = if range == b"*" {
                Some(0)
            } else if range.eq_ignore_ascii_case(offer) {
                Some(usize::MAX)
            } else {
                std::iter::successors(truncate(range), |shorter| truncate(shorter))
                    .find(|shorter| shorter.eq_ignore_ascii_case(offer))
                    .map(<[u8]>::len)
            };
            assert_eq!(lookup_rank(range, offer), expected, "{range:?} / {offer:?}");
        };
        let mut level = vec![String::from("en"), String::from("x")];
        let mut tags = level.clone();
        for _ in 0..4 {
            level = level
                .iter()
                .flat_map(|prefix| {
                    ["a", "2", "cC", "abc", "abcdefgh"].map(|subtag| format!("{prefix}-{subtag}"))
                })
                .collect();
            tags.extend(level.iter().cloned());
        }
        for tag in &tags {
            let range = tag.as_bytes();
            let upper = tag.to_ascii_uppercase();
            // Every byte prefix covers exact, truncation and non-boundary cases.
            for end in 1..=range.len() {
                check(range, &upper.as_bytes()[..end]);
            }
            check(range, b"unrelated");
            check(range, b"*");
            check(b"*", range);
        }
        check(b"*", b"*");
    }
}
