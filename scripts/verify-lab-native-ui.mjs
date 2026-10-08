#!/usr/bin/env node
/**
 * Current native UI receiver: preserves PR65's displayed comparison while the
 * user drafts a new pair, and checks actual report bytes and rendered meaning.
 *
 * node scripts/verify-lab-native-ui.mjs API_BINARY REPO_ROOT UI_ROOT OUTPUT_DIR
 * Set PLAYWRIGHT_MODULE, MARKED_MODULE and CHROMIUM_EXECUTABLE as documented in
 * docs/receipts/lab-ended-branch-20261008/native-receiving/README.md.
 *
 * The original clear-on-selection receiver is retained byte-for-byte in
 * docs/receipts/lab-report-20261008/native-receiving/historical/.
 */
await import("./verify-lab-accepted-ui.mjs");
