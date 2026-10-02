# SteamLogger documentation site

The full documentation for [SteamLogger](../README.md), built with
[Docusaurus 3](https://docusaurus.io/).

Published at **https://havaianasdestruido.github.io/steamlogger/**

## Local development

```bash
cd website
npm install
npm start          # dev server with hot reload on http://localhost:3000/steamlogger/
```

## Commands

| Command | What it does |
| --- | --- |
| `npm start` | Development server with hot reload |
| `npm run build` | Production build into `build/` |
| `npm run serve` | Serve the production build locally |
| `npm run check:api` | Fail if any `pub` item in `../src` is missing from `docs/reference/` |
| `npm run check` | `check:api` + `build` — what CI runs |
| `npm run clear` | Clear the Docusaurus cache |
| `npm run deploy` | Publish to the `gh-pages` branch (needs `GIT_USER`) |

## Structure

```text
docs/
  intro.mdx                 what SteamLogger is, doc map
  getting-started/          installation · quickstart · running continuously
  configuration.mdx         every TOML key and environment variable
  output-format.mdx         the steamlog.json schema
  architecture/             overview · poll loop · session lifecycle · enrichment · resilience
  reference/                one page per module, every public item
  guides/                   jq recipes · using the crate as a library
  contributing/             development · testing · documentation workflow
  troubleshooting.mdx       symptom-first diagnostics
src/
  components/SourceLink/    badge linking a page to the file it documents
  theme/MDXComponents.js    registers <SourceLink /> globally
scripts/
  check-api-coverage.mjs    docs/code drift guard
```

Writing conventions, link rules and the deployment setup are documented in
[Contributing → Documentation workflow](./docs/contributing/documentation.mdx).

## Deployment

`.github/workflows/docs.yml` builds this site on every push to `main` that
touches `website/` or `src/`, and deploys it to GitHub Pages. Pull requests get
a build-only check, so broken links (`onBrokenLinks: 'throw'`) and undocumented
public items fail the PR.
