export default {
  title: 'WebR Framework',
  description: 'A Spring Boot-inspired web framework for Rust',
  base: '/webr/',
  themeConfig: {
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/quick-start' }
    ],
    sidebar: [
      { text: 'Quick Start', link: '/quick-start' },
      { text: 'Configuration', link: '/configuration' },
      { text: 'Controllers & Routing', link: '/controllers-routing' },
      { text: 'Dependency Injection', link: '/dependency-injection' },
      { text: 'Middleware', link: '/middleware' },
      { text: 'Request Handling', link: '/request-handling' },
      { text: 'Response & Error', link: '/response-error' },
      { text: 'File Upload & SSE', link: '/file-upload-sse' },
      { text: 'Database', link: '/database' },
      { text: 'Cache', link: '/cache' }
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/xgpxg/webr' }
    ]
  }
}
