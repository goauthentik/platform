import typescript from "@rollup/plugin-typescript";
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";

export default {
  input: {
    background: "src/background.ts",
    // popup: 'src/popup.ts'
  },
  output: {
    dir: "dist",
    format: "es",
  },
  plugins: [typescript(), resolve(), commonjs()],
};
