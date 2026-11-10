//! Selection rules layered over existing grammar parsers, not a field parser.

use super::{Choice, LanguageMatching};
use parse_net::{
    Limits, media,
    negotiate::{MalformedPolicy, Negotiator, Outcome, parse_weighted},
};

struct MediaRange<'a> {
    kind: u8,
    type_: &'a [u8],
    subtype: &'a [u8],
    params: Vec<(&'a [u8], Vec<u8>)>,
    quality: u16,
}

fn range<'a>(value: &'a [u8], limits: &Limits) -> Result<MediaRange<'a>, String> {
    let parsed = media::parse_range(value, limits).map_err(|e| e.to_string())?;
    // The grammar parser validates the whole member. Read all parameters with
    // its parameter parser because parse_range exposes only those before q.
    let start = value.iter().position(|&b| b == b';').unwrap_or(value.len());
    let (params, _) =
        parse_net::param::parse(&value[start..], limits).map_err(|e| e.to_string())?;
    let mut concrete = Vec::new();
    for param in params.iter().filter(|p| !p.name.eq_ignore_ascii_case(b"q")) {
        let mut decoded = vec![0; param.value.len()];
        param
            .value
            .unescape_into(&mut decoded)
            .map_err(|e| e.to_string())?;
        concrete.push((param.name, decoded));
    }
    Ok(MediaRange {
        kind: if parsed.type_() == b"*" {
            0
        } else if parsed.subtype() == b"*" {
            1
        } else {
            2
        },
        type_: parsed.type_(),
        subtype: parsed.subtype(),
        params: concrete,
        quality: parsed.weight().thousandths(),
    })
}

impl MediaRange<'_> {
    fn matches(&self, offer: &media::MediaType<'_>) -> bool {
        (self.type_ == b"*" || self.type_.eq_ignore_ascii_case(offer.type_()))
            && (self.subtype == b"*" || self.subtype.eq_ignore_ascii_case(offer.subtype()))
            && self.params.iter().all(|(name, value)| {
                offer
                    .params()
                    .iter()
                    .find(|param| param.name.eq_ignore_ascii_case(name))
                    .is_some_and(|param| {
                        let actual = param.value;
                        if name.eq_ignore_ascii_case(b"charset") {
                            actual.eq_ignore_ascii_case(value)
                        } else {
                            actual.eq_bytes(value)
                        }
                    })
            })
    }
}

pub(super) fn media(value: Option<&[u8]>, offers: &[&str], limits: Limits) -> Choice {
    let Some(value) = value else {
        return Ok((!offers.is_empty()).then_some(0));
    };
    let list = parse_net::list::parse(value, &limits).map_err(|e| e.to_string())?;
    let ranges = list
        .iter()
        .map(|member| range(member, &limits))
        .collect::<Result<Vec<_>, _>>()?;
    let mut chosen = None;
    for (index, offer) in offers.iter().enumerate() {
        let offer = media::parse(offer.as_bytes(), &Limits::DEFAULT).map_err(|e| e.to_string())?;
        let mut best = None;
        for range in &ranges {
            let specificity = (range.kind, range.params.len());
            if range.matches(&offer) && best.is_none_or(|(rank, _)| specificity > rank) {
                best = Some((specificity, range.quality));
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

fn truncate(range: &[u8]) -> Option<&[u8]> {
    let cut = range.iter().rposition(|&b| b == b'-')?;
    let mut shorter = &range[..cut];
    if let Some(dash) = shorter.iter().rposition(|&b| b == b'-')
        && shorter.len() - dash == 2
    {
        shorter = &shorter[..dash];
    }
    Some(shorter)
}

// Rank is used only for zero-quality exclusions. Positive lookup preferences
// are processed by client priority, never overwritten by lower-priority ranges.
fn lookup_rank(mut range: &[u8], offer: &[u8]) -> Option<usize> {
    if range == b"*" {
        return Some(0);
    }
    if range.eq_ignore_ascii_case(offer) {
        return Some(usize::MAX);
    }
    while let Some(shorter) = truncate(range) {
        range = shorter;
        if range.eq_ignore_ascii_case(offer) {
            return Some(range.len());
        }
    }
    None
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
