#!/usr/bin/env node

import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join, relative } from "node:path";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = join(scriptDirectory, "..");
const migrationDirectory = join(repositoryRoot, "migrations");
const snapshot = JSON.parse(
  await readFile(join(repositoryRoot, "snapshot", "manifest.json"), "utf8"),
);
const allocations = new Map(
  snapshot.eligible_voters.map(({ address, shares_raw }) => [address, shares_raw]),
);
const files = (await readdir(migrationDirectory))
  .filter((file) => file.endsWith(".json"))
  .sort();

if (files.length === 0) {
  throw new Error("no migration vote manifests found");
}

for (const file of files) {
  const path = join(migrationDirectory, file);
  const migration = JSON.parse(await readFile(path, "utf8"));
  const context = relative(repositoryRoot, path);

  if (migration.schema_version !== 1) {
    throw new Error(`${context}: unsupported schema version`);
  }
  if (migration.source_network_passphrase !== "Test SDF Network ; September 2015") {
    throw new Error(`${context}: unexpected source network`);
  }
  if (!/^C[A-Z2-7]{55}$/.test(migration.source_contract_id)) {
    throw new Error(`${context}: invalid source contract ID`);
  }
  if (!Number.isSafeInteger(migration.captured_through_ledger)) {
    throw new Error(`${context}: invalid capture ledger`);
  }
  if (migration.snapshot_sha256 !== snapshot.source_sha256) {
    throw new Error(`${context}: snapshot SHA-256 mismatch`);
  }
  if (migration.snapshot_allocation_digest !== snapshot.allocation_digest) {
    throw new Error(`${context}: allocation digest mismatch`);
  }
  if (!Array.isArray(migration.options) || migration.options.length < 2) {
    throw new Error(`${context}: invalid options`);
  }

  const seen = new Set();
  const optionShares = migration.options.map(() => 0n);
  const optionCounts = migration.options.map(() => 0);
  for (const vote of migration.votes) {
    if (seen.has(vote.voter)) {
      throw new Error(`${context}: duplicate voter ${vote.voter}`);
    }
    seen.add(vote.voter);
    const allocation = allocations.get(vote.voter);
    if (allocation === undefined) {
      throw new Error(`${context}: ineligible voter ${vote.voter}`);
    }
    if (vote.shares_raw !== allocation) {
      throw new Error(`${context}: incorrect shares for ${vote.voter}`);
    }
    if (!Number.isInteger(vote.option) || vote.option < 0 || vote.option >= migration.options.length) {
      throw new Error(`${context}: invalid option for ${vote.voter}`);
    }
    optionShares[vote.option] += BigInt(allocation);
    optionCounts[vote.option] += 1;
  }

  const totalShares = optionShares.reduce((sum, value) => sum + value, 0n);
  if (migration.results.total_voters !== migration.votes.length) {
    throw new Error(`${context}: total voter count mismatch`);
  }
  if (migration.results.total_voted_shares_raw !== totalShares.toString()) {
    throw new Error(`${context}: total voted shares mismatch`);
  }
  if (migration.results.options.length !== migration.options.length) {
    throw new Error(`${context}: option result count mismatch`);
  }
  migration.results.options.forEach((result, option) => {
    if (
      result.option !== option ||
      result.label !== migration.options[option] ||
      result.voter_count !== optionCounts[option] ||
      result.shares_raw !== optionShares[option].toString()
    ) {
      throw new Error(`${context}: result mismatch for option ${option}`);
    }
  });

  console.log(
    `Verified ${context}: ${migration.votes.length} votes, ${totalShares} raw shares`,
  );
}
