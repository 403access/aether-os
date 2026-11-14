import type { SidebarsConfig } from "@docusaurus/plugin-content-docs";

const sidebars: SidebarsConfig = {
  docs: [
    "intro",
    {
      type: "category",
      label: "Getting started",
      collapsed: false,
      items: [
        "getting-started/install",
        "getting-started/first-boot",
        "getting-started/hardware",
        "getting-started/virtual-machine",
      ],
    },
    {
      type: "category",
      label: "Product",
      collapsed: false,
      items: [
        "product/prd",
        "product/personas-and-jobs",
        "product/scope-and-non-goals",
        "product/ux-principles",
        "product/metrics",
      ],
    },
    {
      type: "category",
      label: "Architecture",
      collapsed: false,
      items: [
        "architecture/overview",
        "architecture/hardware-target",
        "architecture/kernel-and-boot",
        "architecture/nixos-base",
        "architecture/compositor",
        "architecture/smithay-backend",
        "architecture/ui-system",
        "architecture/search",
        "architecture/agent-isolation",
        "architecture/collaboration",
        "architecture/audio",
        "architecture/browser",
        "architecture/media",
        "architecture/files-and-storage",
        "architecture/remote-display",
        "architecture/performance-engineering",
      ],
    },
    {
      type: "category",
      label: "Roadmap",
      collapsed: false,
      items: ["roadmap/roadmap", "roadmap/risks", "roadmap/design-system-and-explorer"],
    },
    {
      type: "category",
      label: "Design",
      items: ["design/design-system", "design/explorer"],
    },
    {
      type: "category",
      label: "Operations",
      items: [
        "ops/build-and-release",
        "ops/observability",
        "ops/security-threat-model",
        "ops/target-check",
      ],
    },
    {
      type: "category",
      label: "Reference",
      items: [
        "reference/claims-and-evidence",
        "reference/glossary",
        "reference/faq",
      ],
    },
    {
      type: "category",
      label: "Decisions",
      collapsed: false,
      items: [
        "adr/index",
        "adr/0001-nixos-base",
        "adr/0002-rust-smithay-compositor",
        "adr/0003-xfs-root",
        "adr/0004-isolation-without-pku",
        "adr/0005-shared-memory-ipc",
        "adr/0006-web-stack-bun-typescript-react",
        "adr/0007-remote-display-webrtc-webcodecs",
        "adr/0008-single-pages-deployment",
        "adr/0009-water-design-system",
        "adr/0010-compositor-crate-split",
        "adr/0011-backend-neutral-shell",
        "adr/0012-smithay-standalone-backend",
        "adr/0013-bitmap-text",
        "adr/0014-desktop-app-discovery",
        "adr/0015-index-service-socket",
      ],
    },
    {
      type: "category",
      label: "Contributing",
      items: ["contributing/development", "contributing/testing"],
    },
  ],
};

export default sidebars;
