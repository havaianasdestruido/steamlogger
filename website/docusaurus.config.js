// @ts-check
// Docusaurus configuration for the SteamLogger documentation site.
// Docs: https://docusaurus.io/docs/api/docusaurus-config

import {themes as prismThemes} from 'prism-react-renderer';

const organizationName = 'havaianasdestruido';
const projectName = 'steamlogger';
const repoUrl = `https://github.com/${organizationName}/${projectName}`;

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'SteamLogger',
  tagline: 'Every Steam session — with friends, lobby, server and map — logged to JSON',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
    faster: true,
  },

  // Production URL (GitHub Pages for <org>.github.io/<repo>/).
  url: `https://${organizationName}.github.io`,
  baseUrl: `/${projectName}/`,

  organizationName,
  projectName,
  deploymentBranch: 'gh-pages',
  trailingSlash: false,

  onBrokenLinks: 'throw',
  onBrokenAnchors: 'throw',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  markdown: {
    mermaid: true,
    hooks: {
      onBrokenMarkdownLinks: 'throw',
      onBrokenMarkdownImages: 'throw',
    },
  },

  themes: [
    '@docusaurus/theme-mermaid',
    [
      // Offline search index, built at compile time (no Algolia account needed).
      require.resolve('@easyops-cn/docusaurus-search-local'),
      /** @type {import('@easyops-cn/docusaurus-search-local').PluginOptions} */
      ({
        hashed: true,
        indexBlog: false,
        docsRouteBasePath: '/docs',
        highlightSearchTermsOnTargetPage: true,
        explicitSearchResultPath: true,
      }),
    ],
  ],

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: './sidebars.js',
          editUrl: `${repoUrl}/tree/main/website/`,
          showLastUpdateTime: true,
          breadcrumbs: true,
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
        sitemap: {
          lastmod: 'date',
          changefreq: 'weekly',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      image: 'img/steamlogger-social-card.png',
      colorMode: {
        defaultMode: 'dark',
        respectPrefersColorScheme: true,
      },
      docs: {
        sidebar: {
          hideable: true,
          autoCollapseCategories: false,
        },
      },
      navbar: {
        title: 'SteamLogger',
        logo: {
          alt: 'SteamLogger logo',
          src: 'img/logo.svg',
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'docsSidebar',
            position: 'left',
            label: 'Documentation',
          },
          {
            to: '/docs/reference',
            label: 'Code reference',
            position: 'left',
          },
          {
            to: '/docs/architecture/overview',
            label: 'Architecture',
            position: 'left',
          },
          {
            href: repoUrl,
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Get started',
            items: [
              {label: 'Introduction', to: '/docs/intro'},
              {label: 'Installation', to: '/docs/getting-started/installation'},
              {label: 'Quickstart', to: '/docs/getting-started/quickstart'},
              {label: 'Configuration', to: '/docs/configuration'},
            ],
          },
          {
            title: 'Understand',
            items: [
              {label: 'Architecture', to: '/docs/architecture/overview'},
              {label: 'Session lifecycle', to: '/docs/architecture/session-lifecycle'},
              {label: 'Enrichment pipeline', to: '/docs/architecture/enrichment-pipeline'},
              {label: 'Log format', to: '/docs/output-format'},
            ],
          },
          {
            title: 'Code reference',
            items: [
              {label: 'Crate map', to: '/docs/reference'},
              {label: 'steamlogger::tracker', to: '/docs/reference/tracker'},
              {label: 'steamlogger::steam', to: '/docs/reference/steam-api'},
              {label: 'steamlogger::a2s', to: '/docs/reference/a2s'},
            ],
          },
          {
            title: 'Project',
            items: [
              {label: 'GitHub', href: repoUrl},
              {label: 'Issues', href: `${repoUrl}/issues`},
              {label: 'Contributing', to: '/docs/contributing/development'},
              {label: 'Steam Web API', href: 'https://steamcommunity.com/dev'},
            ],
          },
        ],
        copyright: `SteamLogger is MIT licensed. Docs built with Docusaurus. © ${new Date().getFullYear()}`,
      },
      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: ['rust', 'toml', 'bash', 'json', 'diff', 'ini', 'systemd'],
      },
      mermaid: {
        theme: {light: 'neutral', dark: 'dark'},
      },
      tableOfContents: {
        minHeadingLevel: 2,
        maxHeadingLevel: 4,
      },
    }),
};

export default config;
