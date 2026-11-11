/**
 * Test bootstrap: register a DOM so React components can be rendered with
 * `@testing-library/react` under `bun test`. Imported via `bunfig.toml`.
 */
import { GlobalRegistrator } from "@happy-dom/global-registrator";

if (typeof globalThis.document === "undefined") {
  GlobalRegistrator.register();
}

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
