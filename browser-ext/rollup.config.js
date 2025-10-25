import commonjs from "@rollup/plugin-commonjs";
import resolve from "@rollup/plugin-node-resolve";
import typescript from "@rollup/plugin-typescript";

export default {
    input: {
        background: "src/background/background.ts",
        options: "src/options/options.ts",
        popup: "src/popup/popup.ts",
        content: "src/content/content.ts",
    },
    output: {
        dir: "dist",
        format: "es",
        sourcemap: true,
    },
    plugins: [
        typescript({
            outputToFilesystem: false,
        }),
        resolve(),
        commonjs(),
    ],
    onwarn: function (warning, warn) {
        if (warning.code === "UNRESOLVED_IMPORT") {
            throw Object.assign(new Error(), warning);
        }
        warn(warning);
    },
};
