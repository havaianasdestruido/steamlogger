// @ts-check

/**
 * Hand-written sidebar: the order below is the intended reading order of the
 * documentation, from "what is this" to "every public item in the crate".
 *
 * @type {import('@docusaurus/plugin-content-docs').SidebarsConfig}
 */
const sidebars = {
  docsSidebar: [
    'intro',
    {
      type: 'category',
      label: 'Getting started',
      collapsed: false,
      items: [
        'getting-started/installation',
        'getting-started/quickstart',
        'getting-started/running-continuously',
      ],
    },
    'configuration',
    'output-format',
    {
      type: 'category',
      label: 'Architecture',
      collapsed: false,
      items: [
        'architecture/overview',
        'architecture/poll-loop',
        'architecture/session-lifecycle',
        'architecture/enrichment-pipeline',
        'architecture/resilience',
      ],
    },
    {
      type: 'category',
      label: 'Code reference',
      collapsed: false,
      link: {type: 'doc', id: 'reference/index'},
      items: [
        'reference/main',
        'reference/config',
        'reference/model',
        'reference/tracker',
        'reference/enrich',
        'reference/storage',
        'reference/a2s',
        'reference/steam-api',
        'reference/steam-client',
      ],
    },
    {
      type: 'category',
      label: 'Guides',
      items: ['guides/querying-the-log', 'guides/library-usage'],
    },
    {
      type: 'category',
      label: 'Contributing',
      items: [
        'contributing/development',
        'contributing/testing',
        'contributing/documentation',
      ],
    },
    'troubleshooting',
  ],
};

export default sidebars;
