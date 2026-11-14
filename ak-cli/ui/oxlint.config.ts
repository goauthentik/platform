/**
 * @file Oxlint configuration
 */

const { default: createOxlintConfig } = await import("@goauthentik/oxlint-config");

export default createOxlintConfig({ padding: true });
