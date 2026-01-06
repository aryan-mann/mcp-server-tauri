/**
 * Version information for the MCP Bridge plugin.
 *
 * Reads the version from this package's package.json at runtime.
 * Both packages share the same version (monorepo single-version policy).
 */

import { createRequire } from 'module';

const require = createRequire(import.meta.url),
      pkg = require('../package.json') as { version: string },
      [ major, minor ] = pkg.version.split('.');

/**
 * Full version string (e.g., "0.6.5")
 */
export const PLUGIN_VERSION_FULL = pkg.version;

/**
 * Cargo-compatible version string for Cargo.toml dependencies (e.g., "0.6")
 * This is the major.minor version used in Cargo.toml dependency specifications.
 */
export const PLUGIN_VERSION_CARGO = `${major}.${minor}`;
