#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join, relative } from "node:path";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = join(scriptDirectory, "..");
const sourcePath = join(
  repositoryRoot,
  "snapshot",
  "comet_cpal_flattened_ownership_before_41c898a1.csv",
);
const manifestPath = join(repositoryRoot, "snapshot", "manifest.json");

const source = await readFile(sourcePath);
const text = source.toString("utf8").trimEnd();
const lines = text.split(/\r?\n/);
const headers = lines[0].split(",");
const expectedColumns = [
  "rank",
  "owner_address",
  "owner_type",
  "flattened_total_cpal_raw",
];
for (const column of expectedColumns) {
  if (!headers.includes(column)) {
    throw new Error(`missing required CSV column: ${column}`);
  }
}

const rows = lines.slice(1).map((line, index) => {
  const fields = line.split(",");
  if (fields.length !== headers.length) {
    throw new Error(`row ${index + 2} has ${fields.length} fields; expected ${headers.length}`);
  }
  return Object.fromEntries(headers.map((header, fieldIndex) => [header, fields[fieldIndex]]));
});

const eligible = [];
const excluded = [];
const digestChunks = [Buffer.from("blend-vote-snapshot-v1", "utf8")];
const addresses = new Set();
let total = 0n;
let accountCount = 0;
let contractCount = 0;

for (const row of rows) {
  if (!/^[GC][A-Z2-7]{55}$/.test(row.owner_address)) {
    throw new Error(`invalid Stellar address at rank ${row.rank}`);
  }
  if (addresses.has(row.owner_address)) {
    throw new Error(`duplicate Stellar address: ${row.owner_address}`);
  }
  addresses.add(row.owner_address);

  if (row.owner_type !== "account" && row.owner_type !== "contract") {
    throw new Error(`unsupported owner type at rank ${row.rank}: ${row.owner_type}`);
  }
  const shares = BigInt(row.flattened_total_cpal_raw);
  if (shares < 0n) {
    throw new Error(`negative allocation at rank ${row.rank}`);
  }
  if (shares === 0n) {
    excluded.push({
      rank: Number(row.rank),
      address: row.owner_address,
      address_type: row.owner_type,
      shares_raw: "0",
    });
    continue;
  }

  total += shares;
  if (row.owner_type === "account") accountCount += 1;
  if (row.owner_type === "contract") contractCount += 1;
  const sharesBytes = Buffer.alloc(16);
  let remainingShares = shares;
  for (let byteIndex = 15; byteIndex >= 0; byteIndex -= 1) {
    sharesBytes[byteIndex] = Number(remainingShares & 255n);
    remainingShares >>= 8n;
  }
  digestChunks.push(Buffer.from(row.owner_address, "ascii"), sharesBytes);
  eligible.push({
    rank: Number(row.rank),
    address: row.owner_address,
    address_type: row.owner_type,
    shares_raw: shares.toString(),
  });
}

if (eligible.length !== 434) {
  throw new Error(`expected 434 positive allocations; found ${eligible.length}`);
}
if (total !== 146100619813817n) {
  throw new Error(`unexpected snapshot total: ${total}`);
}

const manifest = {
  schema_version: 1,
  source_csv: relative(repositoryRoot, sourcePath),
  source_sha256: createHash("sha256").update(source).digest("hex"),
  allocation_digest: createHash("sha256")
    .update(Buffer.concat(digestChunks))
    .digest("hex"),
  allocation_digest_encoding:
    "sha256(utf8('blend-vote-snapshot-v1') || each positive row in CSV order: 56-byte ASCII address || signed i128 big-endian raw shares)",
  asset: "BLND:USDC Comet V1 LP share",
  network: "Stellar mainnet",
  decimals: 7,
  source_row_count: rows.length,
  eligible_holder_count: eligible.length,
  eligible_account_count: accountCount,
  eligible_contract_count: contractCount,
  excluded_zero_weight_holder_count: excluded.length,
  total_eligible_shares_raw: total.toString(),
  total_eligible_shares: `${total / 10_000_000n}.${(total % 10_000_000n)
    .toString()
    .padStart(7, "0")}`,
  eligible_voters: eligible,
  excluded_zero_weight_holders: excluded,
};

await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(
  `Wrote ${relative(repositoryRoot, manifestPath)}: ${eligible.length} voters, ${total} raw shares`,
);
