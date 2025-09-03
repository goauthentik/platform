import commonjs from "@rollup/plugin-commonjs";
import resolve from "@rollup/plugin-node-resolve";
import typescript from "@rollup/plugin-typescript";

export default {
    input: {
        background: "src/background.ts",
        options: "src/options.ts",
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
};
