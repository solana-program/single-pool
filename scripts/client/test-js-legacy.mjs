#!/usr/bin/env zx
import 'zx/globals';
import { cliArguments, workingDirectory } from '../utils.mjs';

// Build the client.
cd(path.join(workingDirectory, 'clients', 'js-legacy'));
await $`pnpm install`;
await $`pnpm build`;

// Build the legacy client and run the tests.
cd(path.join(workingDirectory, 'clients', 'js-legacy'));
await $`pnpm install`;
await $`pnpm build`;
await $`pnpm test ${cliArguments()}`;
