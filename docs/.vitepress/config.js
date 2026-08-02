export default {
    title: 'WebR Framework',
    description: 'A Spring Boot-inspired web framework for Rust',
    base: '/webr/',
    locales: {
        root: {
            label: '简体中文',
            lang: 'zh-CN',
            title: 'WebR 框架',
            description: '一个轻量的 Rust Web 框架，基于 Axum 构建',
            themeConfig: {
                logo: '/logo.png',
                siteTitle: false,
                nav: [
                    {text: '首页', link: '/'},
                    {text: '指南', link: '/quick-start'}
                ],
                sidebar: [
                    {text: '快速开始', link: '/quick-start'},
                    {text: '配置', link: '/configuration'},
                    {text: '控制器与路由', link: '/controllers-routing'},
                    {text: '依赖注入', link: '/dependency-injection'},
                    {text: '请求处理', link: '/request-handling'},
                    {text: '响应与错误', link: '/response-error'},
                    {text: '文件上传与下载', link: '/file-upload'},
                    {text: 'SSE', link: '/sse'},
                    {text: '中间件', link: '/middleware'},
                    {text: '数据库', link: '/database'},
                    {text: '缓存', link: '/cache'},
                    {text: '性能报告', link: '/performance'}
                ],
                socialLinks: [
                    {icon: 'github', link: 'https://github.com/xgpxg/webr'}
                ]
            }
        },
        en: {
            label: 'English',
            lang: 'en',
            title: 'WebR Framework',
            description: 'A Spring Boot-inspired web framework for Rust',
            link: '/en/',
            themeConfig: {
                logo: '/logo.png',
                siteTitle: false,
                nav: [
                    {text: 'Home', link: '/en/'},
                    {text: 'Guide', link: '/en/quick-start'}
                ],
                sidebar: [
                    {text: 'Quick Start', link: '/en/quick-start'},
                    {text: 'Configuration', link: '/en/configuration'},
                    {text: 'Controllers & Routing', link: '/en/controllers-routing'},
                    {text: 'Dependency Injection', link: '/en/dependency-injection'},
                    {text: 'Request Handling', link: '/en/request-handling'},
                    {text: 'Response & Error', link: '/en/response-error'},
                    {text: 'File Upload & Download', link: '/en/file-upload'},
                    {text: 'SSE', link: '/en/sse'},
                    {text: 'Middleware', link: '/en/middleware'},
                    {text: 'Database', link: '/en/database'},
                    {text: 'Cache', link: '/en/cache'},
                    {text: 'Performance', link: '/en/performance'}
                ],
                socialLinks: [
                    {icon: 'github', link: 'https://github.com/xgpxg/webr'}
                ]
            }
        }
    }
}
