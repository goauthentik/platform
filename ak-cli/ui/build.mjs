// Bundles each view into one self-contained HTML file under ak-cli/src/mcp/ui, which `ak` embeds
// with include_str!. The output is checked in so building `ak` doesn't need node.
import { readFile, writeFile } from "node:fs/promises";

import esbuild from "esbuild";

const VIEWS = ["grant-request"];
const OUT_DIR = "../src/mcp/ui";

for (const view of VIEWS) {
    const { outputFiles } = await esbuild.build({
        entryPoints: [`src/${view}.ts`],
        bundle: true,
        format: "esm",
        minify: true,
        loader: { ".svg": "text" },
        write: false,
        logLevel: "info",
    });

    const template = await readFile(`src/${view}.html`, "utf8");
    // Escape closing script tags so the bundle can't end the inline <script> early.
    const js = outputFiles[0].text.replaceAll("</script", "<\\/script");

    await writeFile(
        `${OUT_DIR}/${view}.html`,
        template.replace("/* bundle */", () => js),
    );
}
