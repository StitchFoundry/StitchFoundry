# Company credits for hosted StitchBench and StitchShop (design note)

> **Status:** 2026-11-23, DESIGN ONLY, needs review. Nothing here is built; no price changes.
> Raised for the AI credits launch plan (Crew Chief `plan/assistant-credits-launch.md`;
> StitchBench `docs/plan.md` "FA-4 notes", ask 2). Issue 252. Builds on [`cloud.md`](cloud.md)
> (accounts, ledger, holds, `/v1/messages`, `/v1/ai/log`); read its "What it does" first.

## The problem

Today only a signed-in person's own credits pay for an AI call. On hosted StitchBench and
StitchShop that fails twice: staff don't each hold credits, and background or anonymous calls (the
booking page answering a customer's question) have no person's token, so they stop. The shop needs
to pay from one company balance, with limits that make a runaway bill impossible.

## 1. The company balance

**A company is a second kind of wallet**, next to a person's. It has the same credits, the same
price list (`SF_CLOUD_MARKUP`, packs; nothing changes) and the same hold-then-take rule. It never
merges with a person's balance.

- **Owner.** The person who creates it (**Foundry Account › Create a company**, or from the hosted
  shop's setup page) is its `owner`. A company has one or more owners; the last owner can't leave.
- **Members and roles.** `owner` (everything, incl. delete and roles), `admin` (members, caps,
  top-ups, keys), `staff` (spend only). Members are existing Foundry accounts, added by email
  invite (the invitee must have a confirmed email, as for any spend: CLOUD-10).
- **How staff spend.** A staff member signs in as themselves (their own token). A hosted call names
  the company (`x-stitchfoundry-company: <id>`); the service checks membership and that the feature
  is one the shop enabled, then holds and takes credits **from the company wallet**, not the
  person's. The ledger entry records both: `company` (who paid) and `actor` (who asked). A staff
  member's own balance is never touched by a company call, and a company's is never touched by a
  personal call (no silent fall-through either way: a 402 says which wallet was short).
- **Per-member allowance** (optional, set by an admin): credits per day or month per member, so one
  person can't drain the shop. Default: none (the company caps below apply).
- **Top-up.** Owners and admins buy packs with `POST /v1/checkout {pack, company}`; the same
  Stripe Checkout and signed webhook add the credits to the company wallet, once. Staff can't buy.
  The receipt names "Foundry credits" and the company. An optional **move credits from my account**
  (`POST /v1/company/{id}/transfer`, owner/admin, from their own balance) is one-way: credits moved
  in are not moved back out (no laundering of free sign-up credits into a company, and no
  withdrawals), except by a refund through Stripe for a pack.
- **Free offers stay personal.** Sign-up credits, founding bonus and the free monthly sew-out checks
  belong to a person; they are not counted in, or spendable from, a company wallet. Company spend
  does not count against `SF_CLOUD_FREE_BUDGET_CENTS`.
- **Low balance.** Below `SF_CLOUD_COMPANY_LOW` credits (default 500) the service emails the owners
  once per crossing (Resend, as for other mail) and `GET /v1/company/{id}` says `low: true`.

## 2. Background and anonymous calls

A call with **no person** (a queue job, a scheduled summary, the booking page's customer question)
is billed to the company, authenticated by a **company key** instead of a person's token.

- **Company key** (`sfk_…`): made by an owner/admin (`POST /v1/company/{id}/keys {name, features,
  caps?}`), shown once, stored only as a hash, revocable. It lives on the hosted shop's server, never
  in a browser or the booking page's script. A key is limited to the features it lists (e.g.
  `booking.answer`), and each feature has its own fixed server-side recipe (model, system prompt,
  tools, `max_tokens`), as `/v1/sewout` does today: the caller sends only the user's text, so a leaked
  key can't be used as a general Claude proxy.
- **Anonymous end users** (a customer on the booking page) never reach the service: the shop's
  server calls it with the key and passes an opaque `end_user` (a hash of session or IP, never a name
  or email), used only for the limits below.
- **Caps, all checked before the request goes out** (a failure is a 402 or 429 with a message the
  shop can show; nothing is charged):

  | Cap | Default | Notes |
  |---|---|---|
  | Company monthly cap (`monthlyCapCredits`) | 5,000 | all spend (people and keys); owner-set, calendar month UTC; `null` means only the balance limits |
  | Per-key monthly cap | 1,000 | in addition to the company's |
  | Per-call limit (`perCallCapCredits`) | 40 | the hold; also bounds `max_tokens` by feature |
  | Per-feature rate | 30 a minute a key | 429 beyond |
  | Per `end_user` | 20 calls a day, 3 at once | the booking page's abuse guard |
  | Concurrency | 5 at once a company | as the 3 at once an account today |

- **Runaway-cost protection.** Holds are taken from the wallet before the call, so spend can't
  exceed balance. The tools refused in `/v1/messages` (code, fetch, filtered search) stay refused.
  At 80% of a monthly cap the owners are emailed; at 100% calls fail with 402 `company_cap` until
  the next month or an owner raises it. Three consecutive failed or capped calls from one key in a
  minute pause that key for 10 minutes. An owner can **pause the company** (`POST
  /v1/company/{id}/pause`), which stops every key and member call at once; the app and the shop
  show "AI is paused".
- **Costs are logged** in `sf_ai_calls` as today (product `stitchbench`/`stitchshop`, feature,
  tokens, USD), with `company`, `actor` or `key`, so the owner sees per-company spend.

## 3. API, shop integration and migration

**New routes** (all `Authorization: Bearer sfs_…` of an owner/admin unless noted; errors in the
Anthropic error shape, as everywhere in [`cloud.md`](cloud.md)):

| Route | Does |
|---|---|
| `POST /v1/company {name}` | create; caller becomes owner → `{id, name, credits}` |
| `GET /v1/company/{id}` | `{id, name, credits, monthlySpent, monthlyCapCredits, low, paused, members, keys, history}` (any member; staff see no keys) |
| `POST /v1/company/{id}/members {email, role}` / `DELETE …/members/{account}` | invite, change role, remove |
| `POST /v1/company/{id}/settings {monthlyCapCredits?, perCallCapCredits?, memberAllowance?, features?}` | caps and enabled features |
| `POST /v1/company/{id}/keys {name, features, caps?}` / `DELETE …/keys/{keyId}` | make (secret shown once) and revoke a company key |
| `POST /v1/company/{id}/transfer {credits}` | one-way move from the caller's own balance |
| `POST /v1/company/{id}/pause {paused}` | stop or resume all company spend |
| `POST /v1/checkout {pack, company?}` | existing route; with `company`, the credits go to the wallet |
| `GET /v1/account` | gains `companies: [{id, name, role}]` |

**Changed routes.** `POST /v1/messages` and `POST /v1/sewout` accept `x-stitchfoundry-company` (a
person's call, billed to the company). `POST /v1/ai/log` accepts it too. A new
`POST /v1/company/call {feature, input, end_user?}` takes `Authorization: Bearer sfk_…` (the key)
and runs the feature's fixed recipe → the Anthropic answer plus `x-stitchfoundry-credits` (company
balance) and `x-stitchfoundry-spent`. `sfk_` is accepted on no other route.

**How StitchBench and StitchShop call it.**
- *Staff in the hosted app:* the app already holds the person's session token; it adds the company
  header (the account's `companies` says which; one company is chosen by default, a picker when
  there are several). Same error handling as today, plus the "which wallet" 402 message.
- *Background and booking page:* the shop's server holds the `sfk_` key in its own secret store and
  calls `/v1/company/call`; the browser talks only to the shop's server, which adds `end_user` and
  its own rate limit. On 402/429 the booking page shows the shop's fallback ("we'll reply by email")
  rather than an error.
- *Send to StitchShop* (SHOP-01) is not an AI call and is unchanged.

**Ledger migration.** Additive, no downtime, no rewrite of old entries (the ledger is append-only).
The service keeps `sf_state` as a JSON blob and `sf_ledger(id, at, account BIGINT, entry TEXT)` plus
the file-store twin `ledger.jsonl`.

1. New state in the blob (read as empty when absent, so old data loads): `companies`, `members`,
   `company_keys` (hashed), `company_caps`. Company ids come from the same id counter as accounts and
   **never collide with an account id**, so `sf_ledger.account` can hold a company's id for its own
   wallet rows. Nobody has a company before the migration, so no rows need converting.
2. `ALTER TABLE sf_ledger ADD COLUMN IF NOT EXISTS company BIGINT, ADD COLUMN IF NOT EXISTS actor
   BIGINT;` plus `CREATE INDEX IF NOT EXISTS sf_ledger_company ON sf_ledger (company, at)` for the
   monthly-cap sum. Existing rows keep `NULL`; JSON entries gain optional `company`/`actor`/`key`
   fields that old readers ignore. A company wallet's spend rows set `account` = the company id,
   `company` = the same id and `actor` = the person (or null for a key).
3. Entry kinds added: `company_topup`, `company_transfer_in`, `company_transfer_out` (the matching
   person row), `company_spend`, `company_refund`. Existing kinds are unchanged.
4. Rollback: old code ignores the new columns, tables and state fields, so a redeploy of the previous
   version works; company wallets are simply unreachable until redeployed forward.
5. Test mode (CLOUD-13) gets `company` in `/v1/test/provision` and cleanup removes company wallets of a
   run.

Needs tests when built: hold/take against a company wallet, caps and the 80% mail, per-end-user
limits, key revocation and feature fencing, webhook idempotence for company packs, transfer
one-wayness, migration on a copy of a real ledger, and the fuzz and sweep tests for the new routes.

## 4. Open questions for Bruce (each with a recommendation)

1. **Who owns the company: a person or SiteSentrix-style tenant?** Recommend a person-created
   company with owners (above); a billing entity (VAT, invoices) later if a shop asks.
2. **Can staff also be charged to their own balance when the company is out?** Recommend no: never
   fall through silently; show "ask an owner to top up".
3. **Default monthly cap (5,000 credits = $50) and per-call limit (40).** Recommend these, owner
   editable, hard ceiling per company of 50,000 until we have seen real use.
4. **Anonymous booking-page limits (20 a day a customer).** Recommend yes, plus Turnstile or similar
   on the shop's page; the service can't see a real IP behind the shop's server.
5. **Credit transfer person → company.** Recommend one-way only (above); no withdrawals.
6. **Free credits for a new company?** Recommend none: free offers stay per person (abuse via many
   companies), the free budget stays untouched.
7. **Invoices instead of card top-ups for larger shops?** Recommend not now; packs only, revisit when
   a shop asks for a purchase order.
8. **Auto-top-up** (charge a saved card when low). Recommend not in v1: a stored card plus a
   background key is the worst runaway case. Email at low balance instead.
9. **One company key per feature or per shop?** Recommend per feature group (booking, internal), so
   a booking-page leak can't reach internal tools.
10. **Do other Foundry apps (Laser, Pattern, Screen) share company wallets?** Recommend yes by
    design (it is the account service's wallet), but only StitchBench and StitchShop features are
    enabled at launch.
