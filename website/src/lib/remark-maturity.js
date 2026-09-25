// Renders a page's `maturity` frontmatter as labels under its title. GPUI Kit
// is used in production, but its capabilities do not share one track record;
// a label on the page states that where the reader decides whether to use it.
// Each label links to the definitions on the documentation home.

export const MATURITY_LEVELS = ['stable', 'preview', 'experimental', 'showcase-only', 'platform-dependent'];

const labels = {
  en: {
    stable: ['Stable', 'Used by production desktop applications.'],
    preview: ['Preview', 'Usable and documented; the API and edge-case behavior may still change.'],
    experimental: ['Experimental', 'Works with known gaps; validate it for your product before depending on it.'],
    'showcase-only': ['Showcase only', 'Currently used to demonstrate components, not to ship applications.'],
    'platform-dependent': ['Platform-dependent', 'Availability or behavior differs by platform; this page lists the differences.'],
  },
  'zh-CN': {
    stable: ['稳定', '已用于生产环境的桌面应用。'],
    preview: ['预览', '可以使用且有文档，但 API 与边界行为仍可能调整。'],
    experimental: ['实验性', '可以运行但存在已知缺口，依赖前请针对你的产品验证。'],
    'showcase-only': ['仅用于演示', '目前用于 showcase 演示组件，不用于交付应用。'],
    'platform-dependent': ['依赖平台', '可用性或行为因平台而异，本页列出差异。'],
  },
};

const definitions = {
  en: '/docs#maturity',
  'zh-CN': '/zh-CN/docs#成熟度',
};

export function remarkMaturity() {
  return (tree, file) => {
    const levels = file.data?.astro?.frontmatter?.maturity;
    if (!levels) return;

    const path = file.path ?? file.history?.[0] ?? '';
    const lang = /(?:^|[/\\])zh-CN[/\\]/.test(path) ? 'zh-CN' : 'en';
    const list = Array.isArray(levels) ? levels : [levels];
    for (const level of list) {
      if (!MATURITY_LEVELS.includes(level)) {
        throw new Error(`${path}: unknown maturity "${level}"; use one of ${MATURITY_LEVELS.join(', ')}`);
      }
    }

    const heading = tree.children.findIndex((node) => node.type === 'heading' && node.depth === 1);
    const children = list.map((level) => {
      const [text, title] = labels[lang][level];
      return {
        type: 'link',
        // A site-root path: remarkDocLinks, which runs after this plugin,
        // moves it inside the version being built.
        url: definitions[lang],
        title,
        data: { hProperties: { className: ['doc-maturity__label'], dataMaturity: level } },
        children: [{ type: 'text', value: text }],
      };
    });
    tree.children.splice(heading + 1, 0, {
      type: 'paragraph',
      data: { hName: 'p', hProperties: { className: ['doc-maturity'] } },
      children,
    });
  };
}
