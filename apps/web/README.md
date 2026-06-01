# Rentars Web

Next.js 15 frontend for the Rentars decentralized rental platform.

## Quick Start

```bash
# From monorepo root
yarn workspace web dev

# Or from this directory
yarn dev
```

Runs on **http://localhost:3001**

## Scripts

| Command | Description |
|---|---|
| `yarn dev` | Start development server |
| `yarn build` | Production build |
| `yarn test` | Run Vitest tests |
| `yarn storybook` | Start Storybook |
| `yarn build-storybook` | Build static Storybook |

## Environment

Copy `.env.example` to `.env.local` and fill in:

```
NEXT_PUBLIC_API_URL=http://localhost:3000
NEXT_PUBLIC_SUPABASE_URL=...
NEXT_PUBLIC_SUPABASE_ANON_KEY=...
```
