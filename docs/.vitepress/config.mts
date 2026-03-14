import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Orion',
  description: 'A local-first personal AI assistant platform built in Rust',
  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' }],
    ['meta', { name: 'theme-color', content: '#6366f1' }],
  ],
  cleanUrls: true,
  lastUpdated: true,
  ignoreDeadLinks: [
    /localhost/,
  ],

  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'Orion',

    nav: [
      { text: 'Guide', link: '/getting-started/installation' },
      { text: 'Concepts', link: '/concepts/memory' },
      { text: 'API', link: '/api-reference/overview' },
      { text: 'Crates', link: '/crates/agent-sdk' },
    ],

    sidebar: {
      '/': [
        {
          text: 'Introduction',
          items: [
            { text: 'What is Orion?', link: '/' },
            { text: 'Architecture', link: '/architecture' },
          ],
        },
        {
          text: 'Getting Started',
          items: [
            { text: 'Installation', link: '/getting-started/installation' },
            { text: 'Project Setup', link: '/getting-started/initialization' },
            { text: 'Configuration', link: '/getting-started/configuration' },
            { text: 'Quick Start', link: '/getting-started/quickstart' },
          ],
        },
        {
          text: 'Core Concepts',
          items: [
            { text: 'Memory', link: '/concepts/memory' },
            { text: 'Sessions', link: '/concepts/sessions' },
            { text: 'Skills', link: '/concepts/skills' },
            { text: 'Cron & Scheduling', link: '/concepts/cron' },
            { text: 'Vault', link: '/concepts/vault' },
            { text: 'Agent Tools', link: '/concepts/tools' },
          ],
        },
        {
          text: 'Integrations',
          items: [
            { text: 'Web UI', link: '/integrations/web-ui' },
            { text: 'Telegram Bot', link: '/integrations/telegram' },
            { text: 'WebSocket', link: '/integrations/websocket' },
          ],
        },
        {
          text: 'API Reference',
          items: [
            { text: 'Overview', link: '/api-reference/overview' },
            { text: 'Chat', link: '/api-reference/chat' },
            { text: 'Sessions', link: '/api-reference/sessions' },
            { text: 'Memory', link: '/api-reference/memory' },
            { text: 'Health', link: '/api-reference/health' },
          ],
        },
        {
          text: 'CLI Reference',
          link: '/cli-reference',
        },
        {
          text: 'Crate Reference',
          collapsed: true,
          items: [
            { text: 'agent-sdk', link: '/crates/agent-sdk' },
            { text: 'orion-core', link: '/crates/orion-core' },
            { text: 'orion-memory', link: '/crates/orion-memory' },
            { text: 'orion-vault', link: '/crates/orion-vault' },
            { text: 'orion-session', link: '/crates/orion-session' },
            { text: 'orion-skills', link: '/crates/orion-skills' },
            { text: 'orion-cron', link: '/crates/orion-cron' },
            { text: 'orion-agent', link: '/crates/orion-agent' },
            { text: 'orion-gateway', link: '/crates/orion-gateway' },
            { text: 'orion-telegram', link: '/crates/orion-telegram' },
          ],
        },
      ],
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/gabrieleventuri/orion-rs' },
    ],

    search: {
      provider: 'local',
    },

    editLink: {
      pattern: 'https://github.com/gabrieleventuri/orion-rs/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Built with Rust and Claude.',
    },
  },

  markdown: {
    lineNumbers: true,
  },
})
