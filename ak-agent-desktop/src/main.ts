import "./elements/app-shell";

// @goauthentik/theme switches to dark tokens via html[data-theme="dark"]
const dark = matchMedia("(prefers-color-scheme: dark)");
const applyTheme = () => (document.documentElement.dataset.theme = dark.matches ? "dark" : "light");
applyTheme();
dark.addEventListener("change", applyTheme);
