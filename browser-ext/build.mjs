import esbuild from "esbuild";

const options = {
    entryPoints: [
        { in: "src/background/background.ts", out: "background" },
        { in: "src/options/options.ts", out: "options" },
        { in: "src/content/content.ts", out: "content" },
    ],
    bundle: true,
    outdir: "dist",
    format: "esm",
    sourcemap: true,
    logLevel: "info",
};

if (process.argv.includes("--watch")) {
    const ctx = await esbuild.context(options);
    await ctx.watch();
} else {
    await esbuild.build(options);
}
