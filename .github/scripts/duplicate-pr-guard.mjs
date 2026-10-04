import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const API_ROOT = 'https://api.github.com';
const LABEL = 'duplicate-candidate';
const MARKER = '<!-- duplicate-pr-guard -->';

/** Pull issue numbers from GitHub closing keywords and explicit issue branches. */
export function linkedIssues(pull, repositoryFullName) {
  const repository = repositoryFullName.toLowerCase();
  const [repoOwner, repoName] = repository.split('/');
  const issues = new Set();
  const add = (owner, name, number) => {
    const issue = Number(number);
    if (!Number.isSafeInteger(issue) || issue < 1) return;
    if (owner && name && `${owner}/${name}`.toLowerCase() !== repository) return;
    issues.add(issue);
  };

  const closing = /\b(?:close[sd]?|fix(?:es|ed)?|resolve[sd]?)\s+(?:(?<owner>[A-Za-z0-9_.-]+)\/(?<repo>[A-Za-z0-9_.-]+)#(?<cross>\d+)|#(?<local>\d+)|https?:\/\/github\.com\/(?<urlOwner>[A-Za-z0-9_.-]+)\/(?<urlRepo>[A-Za-z0-9_.-]+)\/issues\/(?<urlNumber>\d+))/gi;
  for (const match of String(pull.body ?? '').matchAll(closing)) {
    const g = match.groups;
    if (g.urlNumber) add(g.urlOwner, g.urlRepo, g.urlNumber);
    else if (g.cross) add(g.owner, g.repo, g.cross);
    else add(repoOwner, repoName, g.local);
  }

  const branch = String(pull.head?.ref ?? '');
  for (const match of branch.matchAll(/(?:^|[\/_.-])issues?[\/_.-]?#?(\d+)(?=$|[\/_.-])/gi)) {
    add(repoOwner, repoName, match[1]);
  }
  for (const match of branch.matchAll(/(?:^|[\/_.-])#(\d+)(?=$|[\/_.-])/g)) {
    add(repoOwner, repoName, match[1]);
  }
  return [...issues].sort((a, b) => a - b);
}

/** Return later open PRs and the earliest open PR for each shared issue. */
export function findDuplicateCandidates(openPulls, repositoryFullName) {
  const ordered = [...openPulls].sort((a, b) => {
    const left = Date.parse(a.created_at ?? '') || 0;
    const right = Date.parse(b.created_at ?? '') || 0;
    return left - right || Number(a.number) - Number(b.number);
  });
  const byIssue = new Map();
  for (const pull of ordered) {
    for (const issue of linkedIssues(pull, repositoryFullName)) {
      if (!byIssue.has(issue)) byIssue.set(issue, []);
      byIssue.get(issue).push(pull);
    }
  }

  const candidates = new Map();
  for (const [issue, pulls] of byIssue) {
    if (pulls.length < 2) continue;
    const earlier = pulls[0];
    for (const duplicate of pulls.slice(1)) {
      if (!candidates.has(duplicate.number)) {
        candidates.set(duplicate.number, { pull: duplicate, issues: new Map() });
      }
      candidates.get(duplicate.number).issues.set(issue, earlier);
    }
  }
  return [...candidates.values()].sort((a, b) => Number(a.pull.number) - Number(b.pull.number));
}

function repoPath(fullName) {
  const [owner, repo] = fullName.split('/');
  if (!owner || !repo || fullName.split('/').length !== 2) {
    throw new Error(`Invalid repository name: ${fullName}`);
  }
  return `/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}`;
}

async function request(token, endpoint, method = 'GET', body) {
  const headers = {
    Accept: 'application/vnd.github+json',
    Authorization: `Bearer ${token}`,
    'X-GitHub-Api-Version': '2022-11-28',
  };
  const options = { method, headers };
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
    options.body = JSON.stringify(body);
  }
  const response = await fetch(`${API_ROOT}${endpoint}`, options);
  const text = await response.text();
  let data = null;
  if (text) {
    try { data = JSON.parse(text); } catch { data = text; }
  }
  if (!response.ok) {
    const error = new Error(`${method} ${endpoint} failed (${response.status}): ${typeof data === 'string' ? data : JSON.stringify(data)}`);
    error.status = response.status;
    throw error;
  }
  return data;
}

async function listAll(token, endpoint) {
  const results = [];
  for (let page = 1; ; page += 1) {
    const separator = endpoint.includes('?') ? '&' : '?';
    const rows = await request(token, `${endpoint}${separator}per_page=100&page=${page}`);
    if (!Array.isArray(rows)) throw new Error(`Expected an array from ${endpoint}`);
    results.push(...rows);
    if (rows.length < 100) return results;
  }
}

async function ensureLabel(token, repository) {
  const base = repoPath(repository);
  try {
    await request(token, `${base}/labels/${encodeURIComponent(LABEL)}`);
    return;
  } catch (error) {
    if (error.status !== 404) throw error;
  }
  try {
    await request(token, `${base}/labels`, 'POST', {
      name: LABEL,
      color: 'd73a4a',
      description: 'Potential duplicate pull request; review before merging.',
    });
  } catch (error) {
    if (error.status !== 422) throw error;
    // A concurrent run may have created the label; confirm instead of retrying the write.
    await request(token, `${base}/labels/${encodeURIComponent(LABEL)}`);
  }
}

function commentText(candidate) {
  const lines = [...candidate.issues.entries()]
    .sort(([left], [right]) => left - right)
    .map(([issue, earlier]) => `- Issue #${issue}: earlier open PR [#${earlier.number}](${earlier.html_url})`);
  return [
    'Potential duplicate candidate: this open PR shares linked issue(s) with an earlier open PR.',
    '',
    ...lines,
    '',
    'This guard flags candidates for human review and never closes PRs.',
    '',
    MARKER,
  ].join('\n');
}

async function markCandidate(token, repository, candidate) {
  const base = repoPath(repository);
  const number = candidate.pull.number;
  await request(token, `${base}/issues/${number}/labels`, 'POST', { labels: [LABEL] });

  const comments = await listAll(token, `${base}/issues/${number}/comments`);
  const existing = comments.find((comment) =>
    comment.user?.login === 'github-actions[bot]' && String(comment.body ?? '').includes(MARKER));
  const body = commentText(candidate);
  if (existing) {
    if (existing.body !== body) {
      await request(token, `${base}/issues/comments/${existing.id}`, 'PATCH', { body });
    }
  } else {
    await request(token, `${base}/issues/${number}/comments`, 'POST', { body });
  }
}

async function main() {
  const token = process.env.GITHUB_TOKEN;
  if (!token) throw new Error('GITHUB_TOKEN is not available');
  const eventPath = process.env.GITHUB_EVENT_PATH;
  if (!eventPath) throw new Error('GITHUB_EVENT_PATH is not available');
  const event = JSON.parse(fs.readFileSync(eventPath, 'utf8'));
  const repository = String(event.repository?.full_name ?? process.env.GITHUB_REPOSITORY ?? '');
  const current = event.pull_request;
  if (!current?.number) throw new Error('Event has no pull_request payload');

  const pullsEndpoint = `${repoPath(repository)}/pulls?state=open&sort=created&direction=asc`;
  const openPulls = await listAll(token, pullsEndpoint);
  if (!openPulls.some((pull) => Number(pull.number) === Number(current.number))) {
    // GitHub's list endpoint can briefly lag the opened/reopened webhook.
    openPulls.push(current);
  }
  const candidates = findDuplicateCandidates(openPulls, repository);
  if (candidates.length === 0) {
    console.log(`No duplicate issue targets among ${openPulls.length} open PR(s) in ${repository}.`);
    return;
  }

  await ensureLabel(token, repository);
  for (const candidate of candidates) {
    await markCandidate(token, repository, candidate);
    const issues = [...candidate.issues.keys()].sort((a, b) => a - b).map((n) => `#${n}`).join(', ');
    console.log(`Flagged PR #${candidate.pull.number} for ${issues}.`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Duplicate PR guard failed: ${error.message}`);
    process.exitCode = 1;
  });
}
