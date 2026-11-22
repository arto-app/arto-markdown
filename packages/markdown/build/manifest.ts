import { createHash } from "crypto";
import fs from "fs";
import path from "path";
import type { Plugin } from "vite";

/** Which file of the build is which, before it is hashed. */
export interface BundleLayout {
  stylesheet: string;
  runtime: string;
  htmlEmbedFrame: string;
  /** Each library bundle, under the name `arto_markdown::Library::name` gives it. */
  libraries: Record<string, string>;
}

export interface BundleFile {
  file: string;
  /**
   * `sha256-` and the base64 of the file's SHA-256, which is what both a
   * `<script integrity>` and a `Content-Security-Policy` hash source take.
   */
  integrity: string;
}

export interface BundleManifest {
  version: 1;
  stylesheet: BundleFile;
  runtime: BundleFile;
  htmlEmbedFrame: BundleFile;
  libraries: Record<string, BundleFile>;
}

function describe(dir: string, file: string): BundleFile {
  const source = path.join(dir, file);
  if (!fs.existsSync(source)) {
    throw new Error(`the build wrote no ${file} to describe in the manifest`);
  }
  const digest = createHash("sha256").update(fs.readFileSync(source)).digest("base64");
  return { file, integrity: `sha256-${digest}` };
}

/** The manifest of the bundles `layout` names, as they are in `dir`. */
export function bundleManifest(dir: string, layout: BundleLayout): BundleManifest {
  return {
    version: 1,
    stylesheet: describe(dir, layout.stylesheet),
    runtime: describe(dir, layout.runtime),
    htmlEmbedFrame: describe(dir, layout.htmlEmbedFrame),
    libraries: Object.fromEntries(
      Object.entries(layout.libraries).map(([name, file]) => [name, describe(dir, file)]),
    ),
  };
}

/**
 * Write `manifest.json` into the output once the build of `environment`,
 * the last one, has finished.
 *
 * It runs before the copies into the crate are made (`order: "pre"`,
 * sequential), so the manifest goes with the bundles it describes.
 */
export function bundleManifestPlugin(environment: string, layout: BundleLayout): Plugin {
  let outDir = "";
  return {
    name: "bundle-manifest",
    apply: "build",
    applyToEnvironment: (candidate) => candidate.name === environment,
    configResolved(config) {
      outDir = path.resolve(config.root, config.build.outDir);
    },
    closeBundle: {
      order: "pre",
      sequential: true,
      handler() {
        const manifest = bundleManifest(outDir, layout);
        fs.writeFileSync(
          path.join(outDir, "manifest.json"),
          `${JSON.stringify(manifest, null, 2)}\n`,
        );
      },
    },
  };
}
