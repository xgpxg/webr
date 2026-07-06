export default {
  title: 'WebR Framework',
  description: 'A Spring Boot-inspired web framework for Rust',
  base: '/webr/',
  themeConfig: {
    nav: [
      { text: '首页', link: '/' },
      { text: '指南', link: '/quick-start' }
    ],
    sidebar: [
      { text: '快速开始', link: '/quick-start' },
      { text: '配置', link: '/configuration' },
      { text: '控制器与路由', link: '/controllers-routing' },
      { text: '依赖注入', link: '/dependency-injection' },
      { text: '中间件', link: '/middleware' },
      { text: '请求处理', link: '/request-handling' },
      { text: '响应与错误', link: '/response-error' },
      { text: '文件上传与 SSE', link: '/file-upload-sse' },
      { text: '数据库', link: '/database' },
      { text: '缓存', link: '/cache' }
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/xgpxg/webr' }
    ]
  }
}
