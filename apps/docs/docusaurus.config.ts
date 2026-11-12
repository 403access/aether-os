import type { Config } from "@docusaurus/types";
import type { Options as PresetOptions } from "@docusaurus/preset-classic";
import { themes as prismThemes } from "prism-react-renderer";

const docsBaseUrl = process.env.DOCS_BASE_URL ?? "/docs/";
const siteBase = docsBaseUrl.replace(/\/docs\/$/, "/");
const isDev = process.env.NODE_ENV !== "production";

const config: Config = {
  title: "Aether OS",
  tagline: "A single-target Linux workstation OS: compositor, zero-copy IPC, isolated agents.",
  favicon: "img/favicon.svg",

  url: "https://olivermolnar.github.io",
  baseUrl: docsBaseUrl,
  organizationName: "olivermolnar",
  projectName: "aether-os",
  trailingSlash: true,

  onBrokenLinks: "warn",
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: "warn",
    },
  },

  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  presets: [
    [
      "classic",
      {
        docs: {
          routeBasePath: "/",
          sidebarPath: "./sidebars.ts",
          editUrl: undefined,
          showLastUpdateTime: false,
        },
        blog: false,
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies PresetOptions,
    ],
  ],

  themeConfig: {
    image: "img/social-card.svg",
    colorMode: {
      defaultMode: "dark",
      disableSwitch: true,
      respectPrefersColorScheme: false,
    },
    navbar: {
      title: "Aether OS",
      logo: {
        alt: "Aether OS",
        src: "img/logo.svg",
      },
      items: [
        { type: "docSidebar", sidebarId: "docs", position: "left", label: "Documentation" },
        { to: "/roadmap", position: "left", label: "Roadmap" },
        { to: "/reference/claims-and-evidence", position: "left", label: "Evidence" },
        {
          href: isDev ? "http://localhost:4321/" : siteBase,
          label: "Marketing site",
          position: "right",
        },
        {
          href: "https://github.com/olivermolnar/aether-os",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Product",
          items: [
            { label: "Overview", to: "/" },
            { label: "Requirements", to: "/product/prd" },
            { label: "Roadmap", to: "/roadmap" },
          ],
        },
        {
          title: "Architecture",
          items: [
            { label: "System overview", to: "/architecture/overview" },
            { label: "Agent isolation", to: "/architecture/agent-isolation" },
            { label: "Performance", to: "/architecture/performance-engineering" },
          ],
        },
        {
          title: "Project",
          items: [
            { label: "Development", to: "/contributing/development" },
            { label: "Testing", to: "/contributing/testing" },
            { label: "Claims & evidence", to: "/reference/claims-and-evidence" },
          ],
        },
      ],
      copyright: `MIT © 2026 Aether OS contributors. Built with Docusaurus and the Water design system.`,
    },
    prism: {
      theme: prismThemes.oneDark,
      darkTheme: prismThemes.oneDark,
      additionalLanguages: ["rust", "bash", "nix", "toml", "json"],
    },
    tableOfContents: {
      minHeadingLevel: 2,
      maxHeadingLevel: 3,
    },
  } satisfies Config["themeConfig"],

  themes: [
    [
      require.resolve("@easyops-cn/docusaurus-search-local"),
      {
        hashed: true,
        language: ["en"],
        indexDocs: true,
        docsRouteBasePath: "/",
        highlightSearchTermsOnTargetPage: true,
      },
    ],
  ],
};

export default config;
