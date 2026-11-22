import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { defineConfig } from "vitepress";

const base = "/slop-warded-router/";

// VitePress fails the build on a link to a missing page. This also fails it on a link to a
// missing heading of an existing page, which a renamed heading would otherwise leave behind.
function checkFragments(outDir: string) {
  const ids = new Map<string, Set<string>>();
  const links: { page: string; target: string; fragment: string }[] = [];
  for (const entry of readdirSync(outDir, { recursive: true, encoding: "utf8" })) {
    const page = entry.replaceAll("\\", "/");
    if (!page.endsWith(".html")) continue;
    const html = readFileSync(path.join(outDir, entry), "utf8");
    ids.set(page, new Set(Array.from(html.matchAll(/\sid="([^"]*)"/g), (match) => match[1])));
    for (const [, href] of html.matchAll(/\shref="([^"]*#[^"]*)"/g)) {
      const [address, fragment] = href.split("#", 2);
      if (!fragment || /^[a-z][a-z0-9+.-]*:/i.test(address)) continue;
      const target = address.startsWith(base)
        ? address.slice(base.length) || "index.html"
        : address
          ? path.posix.join(path.posix.dirname(page), address)
          : page;
      links.push({ page, target, fragment: decodeURIComponent(fragment) });
    }
  }
  const broken = links.filter(({ target, fragment }) => !ids.get(target)?.has(fragment));
  if (broken.length > 0) {
    const list = broken.map(({ page, target, fragment }) => `${page}: ${target}#${fragment}`);
    throw new Error(`Links to missing headings:\n${list.join("\n")}`);
  }
}

// The version the site describes. The Pages workflow builds the site from that release's tag,
// whose Cargo.toml names it; the release text check keeps the pages' pins in step with it.
const manifest = readFileSync(new URL("../../Cargo.toml", import.meta.url), "utf8");
const version = /^version = "([^"]+)"$/m.exec(manifest)?.[1];
if (!version) throw new Error("Cargo.toml names no package version");

const repository = "https://github.com/viell-dev/slop-warded-router";
const api = `https://docs.rs/warded/${version}/warded/`;

type Item = { text: string; link: string };
const start: Item[] = [
  { text: "Getting started", link: "/getting-started" },
  { text: "How selection works", link: "/selection" },
];
const guides: Item[] = [
  { text: "Paths and captures", link: "/paths" },
  { text: "Methods, headers and query parameters", link: "/conditions" },
  { text: "Transport security", link: "/transport-security" },
  { text: "Content negotiation", link: "/negotiation" },
  { text: "Tower and Axum", link: "/tower" },
  { text: "Custom matchers", link: "/custom-matchers" },
];
const help: Item[] = [
  { text: "Troubleshooting", link: "/troubleshooting" },
  { text: "Scope and limits", link: "/limits" },
];
const releases: Item[] = [
  { text: "Versions", link: "/versions" },
  { text: "Upgrading", link: "/upgrading" },
  { text: "Changelog", link: "/changelog" },
];

export default defineConfig({
  title: "Warded",
  description:
    "Rust request routing over the complete request head: method, path, headers, query, HTTP version and trusted HTTPS context.",
  lang: "en",
  base,
  buildEnd: ({ outDir }) => checkFragments(outDir),
  head: [["meta", { name: "theme-color", content: "#3c6e47" }]],
  sitemap: { hostname: "https://viell-dev.github.io/slop-warded-router/" },
  // Pages and their example checks; everything else in this directory builds or tests them.
  srcExclude: ["**/node_modules/**"],
  markdown: {
    // Show `{{` in code spans as written; Vue would otherwise read it as an interpolation.
    config(md) {
      md.core.ruler.push("code_inline_v_pre", (state) => {
        for (const token of state.tokens)
          for (const child of token.children ?? [])
            if (child.type === "code_inline") child.attrSet("v-pre", "");
      });
    },
  },
  themeConfig: {
    nav: [
      { text: "Guide", items: [...start, ...guides] },
      { text: "Help", items: help },
      { text: "API reference", link: api },
      { text: version, items: releases },
    ],
    sidebar: [
      { text: "Start here", items: start },
      { text: "Guides", items: guides },
      { text: "Help", items: help },
      {
        text: "Reference",
        items: [{ text: "API reference (docs.rs)", link: api }, ...releases],
      },
    ],
    search: { provider: "local" },
    outline: { level: [2, 3] },
    socialLinks: [{ icon: "github", link: repository }],
    // Changes start on main, where pull requests go; a fix to the released site is then also
    // applied to its correction branch, as the release procedure describes.
    editLink: {
      pattern: `${repository}/edit/main/docs/:path`,
      text: "Suggest a change to this page on main",
    },
    footer: {
      message: `Agent-maintained · <a href="${repository}/blob/${version}/LICENSE-MIT">MIT</a> OR <a href="${repository}/blob/${version}/LICENSE-APACHE">Apache-2.0</a>`,
    },
  },
});
