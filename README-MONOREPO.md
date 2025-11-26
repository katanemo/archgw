# ArchGW Monorepo

This is a Turborepo monorepo containing the Next.js applications and shared packages.

## Structure

```
.
├── apps/
│   ├── www/          # Marketing website
│   └── docs/          # Documentation site
├── packages/
│   ├── ui/            # Shared UI components (Navbar, Footer, Logo, etc.)
│   ├── shared-styles/ # Shared CSS and Tailwind configuration
│   ├── tailwind-config/ # Tailwind configuration
│   └── tsconfig/      # Shared TypeScript configurations
└── turbo.json         # Turborepo configuration
```

## Getting Started

### Install Dependencies

```bash
npm install
```

### Development

Run all apps in development mode:

```bash
npm run dev
```

Or run specific apps:

```bash
# Marketing website (port 3000)
cd apps/www && npm run dev

# Documentation (port 3001)
cd apps/docs && npm run dev
```

### Build

Build all apps:

```bash
npm run build
```

### Lint & Type Check

```bash
npm run lint
npm run typecheck
```

## Shared Packages

### @archgw/ui

Shared React components including:
- `Navbar` - Navigation bar component
- `Footer` - Footer component
- `Logo` - Logo component
- UI components (Button, Dialog, etc.)

### @archgw/shared-styles

Shared CSS styles including:
- Tailwind CSS configuration
- Font definitions (IBM Plex Sans, JetBrains Mono)
- CSS variables for theming

### @archgw/tailwind-config

Shared Tailwind CSS configuration.

### @archgw/tsconfig

Shared TypeScript configurations:
- `base.json` - Base TypeScript config
- `nextjs.json` - Next.js specific config

## Design System

Both apps share the same design system:
- Same fonts (IBM Plex Sans, JetBrains Mono)
- Same color palette
- Same components (Navbar, Footer)
- Same Tailwind configuration

## Notes

- Fonts are stored in each app's `public/fonts/` directory
- Both apps use the same shared components and styles
- The monorepo uses npm workspaces and Turborepo for build orchestration

