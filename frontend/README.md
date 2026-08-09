# Ajo Web App
 
The Ajo web frontend — a **Next.js 15** app (App Router) styled with **Tailwind CSS**,
connecting to the [Ajo Soroban contract](../contracts) via the Stellar SDK and the
Freighter wallet extension.

---

## Features

**Landing page**
- Hero with animated SVG illustration, inline stats ($0 fees, USDC, Open source)
- Trust band, Features (3-column cards), Circles Showcase (dark section with live mock circle cards)
- How It Works, Why On-Chain, Testimonials, FAQ, final CTA card

**Dashboard** (`/dashboard`)
- Dark sidebar — logo, wallet chip, navigation, network indicator, disconnect
- Sticky top bar — greeting, notification bell (payout alert dot), Connect / New Circle
- KPI cards (4-up grid): circles joined, saved this round, upcoming payout, open to join
- Savings chart + My Circles panel (5-column layout)
- Activity feed + Quick actions + test wallet balance card

**Circles** (`/dashboard/circles`)
- Filter by status (Open / Active / Finished)
- CircleCard — name, pool amount, contribution, members, frequency, progress bar

**Create a circle** (`/dashboard/circles/create`)
- Configure name, token, contribution amount, cycle frequency, group size

**Circle detail** (`/dashboard/circles/[id]`)
- Join, contribute, claim payout; live progress bar; rotation order; member list

**Wallet**
- Connect via Freighter browser extension; shortened address pill in sidebar and top bar

---

## Tech stack

| Concern | Choice |
|---------|--------|
| Framework | Next.js 15 (App Router, Turbopack dev) |
| Styling | Tailwind CSS v3 + Satoshi / Newsreader fonts |
| Blockchain | `@stellar/stellar-sdk`, `@stellar/freighter-api` |
| Icons | `lucide-react` |

---

## Getting started

```bash
npm install
cp .env.local.example .env.local   # optional — mock mode works without env vars
npm run dev                        # → http://localhost:3000
```

The app defaults to **mock mode** (`NEXT_PUBLIC_USE_MOCK=true`), rendering the full UI
with sample circles so you can develop without a deployed contract.

### Environment variables

| Variable | Purpose |
|----------|---------|
| `NEXT_PUBLIC_CONTRACT_ID` | Deployed Ajo contract address (Testnet) |
| `NEXT_PUBLIC_NETWORK` | `TESTNET` or `MAINNET` |
| `NEXT_PUBLIC_USE_MOCK` | `true` to use in-memory sample data |
| `NEXT_PUBLIC_SOROBAN_RPC_URL` | Soroban RPC endpoint |
| `NEXT_PUBLIC_HORIZON_URL` | Horizon endpoint |

---

## Project structure

```
frontend/
├── app/
│   ├── page.tsx                  # landing page
│   ├── layout.tsx                # root layout (WalletProvider + ClientShell)
│   └── dashboard/
│       ├── layout.tsx            # sidebar wrapper (DashboardSidebar)
│       ├── page.tsx              # overview (KPI cards, chart, activity feed)
│       └── circles/
│           ├── page.tsx          # circle list
│           ├── create/page.tsx   # create form
│           └── [id]/page.tsx     # circle detail
├── components/
│   ├── ui/                       # Button, Badge, Card primitives
│   ├── layout/                   # Navbar, Footer, DashboardSidebar, ClientShell
│   ├── landing/                  # Hero, TrustBand, Features, CirclesShowcase,
│   │                             #   HowItWorks, WhyOnChain, Testimonials, FAQ
│   ├── circles/                  # CircleCard, CreateCircleForm, MemberList, …
│   └── dashboard/                # SavingsChart, ActivityFeed, CircleList, MetricCard
├── context/                      # WalletContext (Freighter)
├── hooks/                        # useCircle, useAllCircles
├── lib/                          # contract integration, Stellar SDK, mockData, utils
└── types/                        # shared Ajo types + formatters
```

---

## Scripts

```bash
npm run dev          # start dev server (Turbopack)
npm run build        # production build
npm run start        # serve production build
npm run lint         # eslint
npm run type-check   # tsc --noEmit
```

---

## Design system

Both the web app and mobile app share the same token palette:

| Token | Hex | Usage |
|-------|-----|-------|
| `ajo-dark` | `#1E1D1B` | Charcoal — headings, sidebar background |
| `ajo-lime` | `#D47253` | Coral — primary buttons, accents, active states |
| `ajo-surface` | `#FAF8F3` | Warm cream — page background |
| `ajo-muted` | `#73716D` | Secondary text, labels |
| `ajo-green` | `#16A34A` | Active badges, network indicator |
| `ajo-amber` | `#F59E0B` | Payout alerts, warnings |
| `ajo-border` | `#EBE8E1` | Card borders, dividers |

- **Max width:** `max-w-8xl` (88 rem) with generous horizontal padding
- **Radius:** `rounded-3xl` (cards), `rounded-full` (pills/buttons), `rounded-4xl` (2 rem, CTAs)
- **Fonts:** Satoshi (sans) + Newsreader (serif, page/section headings)
