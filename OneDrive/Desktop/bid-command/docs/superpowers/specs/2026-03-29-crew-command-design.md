# Hoags Crew Command — Design Specification

**Date:** 2026-03-29
**Author:** Collin (Hoags Inc) + Claude
**Status:** Approved

## Overview

Enterprise crew operations management system for debris/fuel removal operations during pre-fire season. Manages crew rosters, flexible training/certification tracking, full equipment asset management, insurance/COI compliance, contract site management, dispatch scheduling, and AI-powered document intelligence.

**Primary user:** Collin (Owner) — full command center on desktop/tablet.
**Secondary users:** Field supervisors — lean mobile views for site status, daily reports, inspections, and contract Q&A.

## Technical Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Framework | Next.js 16 (App Router) | Server Components for fast mobile loads, Server Actions for forms, single deployable |
| Database | Postgres (Neon via Vercel Marketplace) | Relational backbone for deeply interconnected crew/cert/equipment/site data |
| ORM | Prisma | Type-safe schema-as-code, migrations, audit-friendly |
| UI | shadcn/ui + Tailwind CSS | Dark ops aesthetic, responsive, accessible components |
| Auth | NextAuth.js | Role-based access, invite codes for supervisors |
| File Storage | Vercel Blob | Cert docs, COIs, contract PDFs, inspection photos, receipts |
| Vector Search | pgvector (Neon) | RAG embeddings stored alongside relational data |
| AI | Vercel AI SDK + AI Gateway | Document Q&A, natural language queries |
| Hosting | Vercel (or self-hosted) | Zero-ops deploy, free tier to start |

## Data Model

### Organization

- `id`, `name`, `legalName`, `cageCode`, `uei`, `samStatus`
- `phone`, `email`, `address`
- `divisions[]` — support for multiple business lines (Fuel Removal, Janitorial, etc.)
- All records carry `createdAt`, `updatedAt`, `createdBy`, `updatedBy` for audit trail

### Personnel

- `id`, `firstName`, `lastName`, `ssn` (encrypted at rest), `dob`, `address`, `phone`, `email`
- `emergencyContacts[]` — name, phone, relationship (multiple per person)
- `status`: Applicant | Onboarding | Active | On Leave | Terminated
- `role`: Owner | Manager | Supervisor | Crew Lead | Crew Member
- `hireDate`, `terminationDate`, `terminationReason`
- `payRate`, `payType` (Hourly | Salary), `overtimeRules`, `scaWageDeterminationId`
- `documents[]` — W-4, I-9, direct deposit, signed policies (uploaded, versioned, timestamped)
- `drugTests[]` — date, type (Pre-Employment | Random | Post-Incident | Reasonable Suspicion), result, lab, chainOfCustodyDoc
- `physicalFitness[]` — date, type, result, provider, doc
- `medicalClearances[]` — date, type, restrictions, expiry, doc
- `backgroundChecks[]` — date, status (Pending | Cleared | Failed), provider, doc
- `availabilityExceptions[]` — startDate, endDate, reason (PTO | Unavailable | Medical)
- `performanceNotes[]` — date, author, category, content
- `incidentReports[]` — date, site, description, severity, witnesses, corrective action, doc

### Certification Types (Admin-Defined)

- `id`, `name`, `category`, `description`
- `expires`: boolean
- `validityPeriod`: number of months (if expires)
- `requiredForRoles[]` — which roles require this cert
- `requiredForContractTypes[]` — which contract types require this cert
- `renewalRequirements` — text describing what's needed to renew
- `isActive`: boolean

### Certification Records

- `id`, `personnelId`, `certTypeId`
- `dateEarned`, `expirationDate`
- `status`: Current | Expired | Pending | Revoked
- `issuingAuthority`, `certNumber`
- `documentId` — uploaded proof
- `notes`

### Training Events

- `id`, `certTypeId` (optional — may not map to a cert)
- `date`, `endDate`, `location`, `instructor`
- `attendees[]` — personnelIds
- `hours`, `topic`, `description`
- `documentId` — sign-in sheet, materials

### Equipment / Assets

- `id`, `assetTag` (internal ID / barcode / QR), `category` (admin-defined)
- `name`, `make`, `model`, `year`, `serialNumber`, `vin` (vehicles)
- `purchaseDate`, `purchasePrice`, `vendor`, `warrantyExpiry`
- `depreciationMethod`: Straight-Line | MACRS
- `usefulLifeMonths`, `salvageValue`
- `ownership`: Owned | Rented | Leased
- `rentalSource`, `rentalRate`, `rentalPeriod` (Daily | Weekly | Monthly)
- `condition`: Excellent | Good | Fair | Poor | Out of Service
- `location`: In Storage | In Shop | Assigned to Site | Assigned to Person
- `assignedSiteId`, `assignedPersonnelId`
- `retiredDate`, `retiredReason`, `disposalMethod`, `disposalValue`

### Asset Categories (Admin-Defined)

- `id`, `name`, `description`
- `inspectionChecklistTemplate[]` — configurable checklist items for pre-use inspections

### Maintenance Work Orders

- `id`, `assetId`
- `type`: Scheduled | Reported Issue | Recall
- `status`: Reported | Assigned | In Progress | Complete | Cancelled
- `reportedBy`, `assignedTo`, `reportedDate`, `completedDate`
- `description`, `resolution`
- `parts[]` — description, quantity, cost
- `laborHours`, `totalCost`
- `nextScheduledDate` (for recurring maintenance)
- `scheduledBy`: Hours | Miles | Calendar

### Fuel Logs

- `id`, `assetId`, `date`
- `gallons`, `costPerGallon`, `totalCost`
- `odometer` (vehicles), `engineHours` (saws/ATVs)
- `location`, `receiptDocId`

### Equipment Inspections

- `id`, `assetId`, `personnelId` (inspector), `date`
- `checklistItems[]` — item name, pass/fail, notes
- `overallResult`: Pass | Fail | Conditional
- `notes`, `photoIds[]`

### Chain of Custody

- `id`, `assetId`, `personnelId`
- `checkedOutDate`, `checkedInDate`
- `conditionAtCheckout`, `conditionAtCheckin`
- `siteId`, `notes`

### Insurance Policies

- `id`, `type` (admin-defined: GL, Workers Comp, Auto, Umbrella, Bonding, etc.)
- `status`: Active | Expired | Shopping | Cancelled
- `carrier`, `agentName`, `agentPhone`, `agentEmail`
- `policyNumber`, `coveragePerOccurrence`, `coverageAggregate`
- `premium`, `paymentSchedule` (Monthly | Quarterly | Annual)
- `paymentHistory[]` — date, amount, method
- `effectiveDate`, `expirationDate`
- `documentId` — uploaded policy doc
- `notes`

### Insurance Shopping (for policies in "Shopping" status)

- `quotes[]` — carrier, premium, coverage, quoteDate, quoteDocId
- `decisionStatus`: Researching | Quotes Received | Decided | Purchased
- `decisionNotes`

### COI Certificates

- `id`, `policyId`, `contractId`
- `recipientName`, `recipientEmail`
- `sentDate`, `documentId`
- `notes`

### Insurance Requirements

- `id`, `contractId`
- `requiredPolicyType`, `minimumCoverage`
- `linkedPolicyId` (null if gap exists)
- `status`: Met | Gap | Pending

### Contracts

- `id`, `solicitationNumber`, `contractNumber`
- `agency`, `contractingOfficerName`, `contractingOfficerEmail`, `contractingOfficerPhone`
- `type`: Firm-Fixed | T&M | IDIQ | BPA
- `performancePeriodStart`, `performancePeriodEnd`
- `optionYears[]` — start, end, exercised boolean
- `contractValue`, `fundedAmount`, `invoicedToDate`
- `status`: Bidding | Awarded | Mobilizing | Active | Demobilizing | Complete | Closed Out
- `division` — which business line
- `notes`

### Sites

- `id`, `contractId`
- `name`, `address`, `gpsLat`, `gpsLon`, `acreage`
- `accessNotes` — directions, gate codes, road conditions
- `hazardNotes` — known hazards, wildlife, terrain
- `scopeDescription`
- `status`: Planned | Active | Demobilizing | Complete

### Crew Assignments

- `id`, `personnelId`, `siteId`
- `role` — role on THIS site (may differ from org role)
- `startDate`, `endDate`
- `status`: Scheduled | Active | Complete

### Equipment Assignments (to sites)

- `id`, `assetId`, `siteId`
- `deployedDate`, `returnedDate`
- `notes`

### Daily Reports

- `id`, `siteId`, `submittedBy`, `date`
- `crewCount`, `totalHours`
- `workAccomplished` — text description
- `weather` — conditions, temperature, wind
- `safetyNotes`
- `incidentOccurred`: boolean
- `incidentDescription`
- `photoIds[]`
- `status`: Draft | Submitted | Reviewed

### Documents

- `id`, `fileName`, `fileType`, `fileSize`, `blobUrl`
- `category`: SOW | Wage Determination | Site Map | Safety Plan | Modification | Invoice | COI | Cert | Receipt | Inspection Photo | Other
- `linkedEntityType`: Contract | Site | Personnel | Policy | Asset
- `linkedEntityId`
- `uploadedBy`, `uploadedAt`
- `version` — incremented on re-upload
- `embeddingStatus`: Pending | Embedded | Failed | Not Applicable

### Document Chunks (for RAG)

- `id`, `documentId`
- `content` — text chunk
- `embedding` — vector (pgvector)
- `chunkIndex` — position in document
- `metadata` — page number, section header

### Notifications

- `id`, `type`: Cert Expiring | Insurance Expiring | Maintenance Due | Daily Report Missing | Contract Award
- `severity`: Info | Warning | Critical
- `title`, `message`
- `linkedEntityType`, `linkedEntityId`
- `recipientId` (null = broadcast to admins)
- `read`: boolean
- `createdAt`

### Audit Log

- `id`, `tableName`, `recordId`
- `action`: Create | Update | Delete
- `changedBy`
- `changedAt`
- `oldValues` (JSON), `newValues` (JSON)

## Architecture

### Project Structure

```
hoags-crew-command/
├── prisma/
│   └── schema.prisma
├── src/
│   ├── app/
│   │   ├── (auth)/
│   │   │   ├── login/page.tsx
│   │   │   └── invite/[code]/page.tsx
│   │   ├── (command)/              # Admin views — Owner/Manager role
│   │   │   ├── layout.tsx          # Sidebar nav, notification bell, user menu
│   │   │   ├── dashboard/page.tsx
│   │   │   ├── personnel/
│   │   │   │   ├── page.tsx        # Roster table
│   │   │   │   ├── [id]/page.tsx   # Profile with tabs
│   │   │   │   ├── new/page.tsx    # Onboarding flow
│   │   │   │   └── compliance/page.tsx  # Cert matrix
│   │   │   ├── training/
│   │   │   │   ├── page.tsx        # Training events list
│   │   │   │   ├── cert-types/page.tsx  # Admin: define cert types
│   │   │   │   └── events/new/page.tsx
│   │   │   ├── equipment/
│   │   │   │   ├── page.tsx        # Asset table
│   │   │   │   ├── [id]/page.tsx   # Asset detail with tabs
│   │   │   │   ├── new/page.tsx
│   │   │   │   ├── work-orders/page.tsx  # Kanban board
│   │   │   │   ├── fleet/page.tsx  # Fleet dashboard
│   │   │   │   └── categories/page.tsx   # Admin: define categories + checklists
│   │   │   ├── insurance/
│   │   │   │   ├── page.tsx        # Policies table + shopping list
│   │   │   │   ├── [id]/page.tsx   # Policy detail
│   │   │   │   ├── new/page.tsx
│   │   │   │   ├── requirements/page.tsx  # Coverage matrix
│   │   │   │   └── coi/page.tsx    # COI tracker
│   │   │   ├── contracts/
│   │   │   │   ├── page.tsx        # Pipeline view
│   │   │   │   ├── [id]/page.tsx   # Contract detail
│   │   │   │   ├── new/page.tsx
│   │   │   │   └── [id]/sites/
│   │   │   │       ├── page.tsx    # Sites list for contract
│   │   │   │       └── [siteId]/page.tsx  # Site detail
│   │   │   ├── dispatch/
│   │   │   │   └── page.tsx        # Calendar dispatch board
│   │   │   ├── documents/
│   │   │   │   ├── page.tsx        # Document library + search
│   │   │   │   └── ask/page.tsx    # RAG query interface
│   │   │   ├── reports/
│   │   │   │   ├── page.tsx        # Report selector
│   │   │   │   ├── financial/page.tsx
│   │   │   │   ├── compliance/page.tsx
│   │   │   │   └── utilization/page.tsx
│   │   │   └── settings/
│   │   │       ├── page.tsx
│   │   │       ├── organization/page.tsx
│   │   │       ├── users/page.tsx
│   │   │       └── alerts/page.tsx
│   │   ├── (field)/                # Supervisor mobile views
│   │   │   ├── layout.tsx          # Bottom tab nav, minimal chrome
│   │   │   ├── my-site/page.tsx    # Current site overview
│   │   │   ├── daily-report/page.tsx
│   │   │   ├── inspections/
│   │   │   │   ├── page.tsx        # Equipment list for inspection
│   │   │   │   └── [assetId]/page.tsx  # Inspection checklist
│   │   │   └── ask/page.tsx        # RAG query
│   │   ├── api/
│   │   │   ├── auth/[...nextauth]/route.ts
│   │   │   ├── chat/route.ts       # RAG query endpoint
│   │   │   ├── alerts/cron/route.ts # Daily alert scan
│   │   │   └── webhooks/route.ts
│   │   ├── layout.tsx              # Root layout
│   │   └── page.tsx                # Redirect to dashboard or login
│   ├── components/
│   │   ├── ui/                     # shadcn/ui primitives
│   │   └── domain/
│   │       ├── stat-card.tsx
│   │       ├── crew-table.tsx
│   │       ├── cert-badge.tsx
│   │       ├── compliance-matrix.tsx
│   │       ├── asset-row.tsx
│   │       ├── work-order-card.tsx
│   │       ├── dispatch-calendar.tsx
│   │       ├── pipeline-board.tsx
│   │       ├── daily-report-form.tsx
│   │       ├── inspection-checklist.tsx
│   │       ├── document-upload.tsx
│   │       └── notification-bell.tsx
│   ├── lib/
│   │   ├── db.ts                   # Prisma client singleton
│   │   ├── auth.ts                 # NextAuth config
│   │   ├── permissions.ts          # Role-based access helpers
│   │   ├── audit.ts                # Audit log writer (wraps Prisma mutations)
│   │   ├── alerts.ts               # Expiration scanner + notification creator
│   │   ├── rag/
│   │   │   ├── ingest.ts           # PDF/DOCX chunking + embedding
│   │   │   ├── search.ts           # Vector similarity search with scope filtering
│   │   │   └── extract.ts          # Text extraction (PDF, DOCX, images via OCR)
│   │   ├── depreciation.ts         # Asset depreciation calculations
│   │   └── encryption.ts           # SSN encryption/decryption helpers
│   ├── actions/                    # Server Actions grouped by domain
│   │   ├── personnel.ts
│   │   ├── training.ts
│   │   ├── equipment.ts
│   │   ├── insurance.ts
│   │   ├── contracts.ts
│   │   ├── dispatch.ts
│   │   ├── documents.ts
│   │   └── reports.ts
│   └── types/
│       └── index.ts                # Shared TypeScript types (derived from Prisma)
├── public/
│   └── icons/                      # Category icons, role badges
├── docs/
│   └── superpowers/specs/          # This spec
├── .env.local                      # Local secrets (gitignored)
├── next.config.ts
├── tailwind.config.ts
├── package.json
└── tsconfig.json
```

### Key Architectural Decisions

1. **Route groups** — `(command)` for admin desktop views, `(field)` for supervisor mobile views. Same codebase, different layouts and permission gates. Route group determines which layout wraps the page.

2. **Server Components by default** — crew lists, cert matrices, asset tables, dispatch boards all render server-side. Fast on supervisor phones with limited bandwidth. Client Components only for interactive elements (calendar drag-and-drop, Kanban boards, forms).

3. **Server Actions for all mutations** — creating crew, logging fuel, submitting daily reports, assigning equipment. Progressive enhancement: forms work even before JS loads.

4. **Role-based access control** — 5 tiers: Owner > Manager > Supervisor > Crew Lead > Crew Member. Each route/action checks role via `permissions.ts`. Supervisors see only their assigned site's data. Owner/Manager see everything.

5. **Audit trail** — `audit.ts` wraps every Prisma create/update/delete. Stores who changed what, when, with old and new values as JSON. Every table gets audited automatically.

6. **Encrypted PII** — SSN encrypted at rest using AES-256-GCM via `encryption.ts`. Decrypted only when explicitly needed (never in list views).

7. **RAG scoped to entities** — documents are linked to contracts/sites via `linkedEntityType`/`linkedEntityId`. Queries from supervisors auto-scope to their assigned site's documents. Admin queries can search everything.

8. **Alert engine** — daily cron job (`/api/alerts/cron`) scans for: certs expiring within 30/60/90 days, insurance expiring, maintenance due, missing daily reports. Creates notification records. Sends email via Resend.

9. **Admin-defined flexibility** — cert types, asset categories, insurance policy types, and inspection checklists are all admin-configurable. No hardcoded business rules that lock you into one way of operating.

10. **Depreciation engine** — `depreciation.ts` calculates current book value using straight-line or MACRS methods based on purchase date, cost, useful life, and salvage value. Updated on demand, not stored (derived data).

## UI Design

### Theme

- **Mode:** Dark by default (zinc-900 backgrounds, zinc-800 cards)
- **Accent:** Amber (#f59e0b) — consistent with Hoags bid wizard aesthetic
- **Type:** Geist Sans for interface text, Geist Mono for IDs, dates, dollar amounts, serial numbers
- **Borders:** Subtle zinc-700 borders, 8px radius on cards
- **Icons:** Lucide icon set via shadcn/ui

### Command Center (Desktop — Owner/Manager)

**Layout:** Fixed sidebar (collapsible) with nav sections: Dashboard, Personnel, Training, Equipment, Insurance, Contracts, Dispatch, Documents, Reports, Settings. Top bar with notification bell and user menu.

**Dashboard**
- 4 stat cards: Active Contracts (count), Crew Deployed (count), Equipment Out (count), Cert Alerts (count, amber if > 0, red if critical)
- Dispatch calendar (week view) — color-coded bars showing crew on sites
- "Expiring Soon" panel — sorted by urgency, links to relevant records
- Recent daily reports feed — latest submissions from supervisors
- Financial summary — revenue/cost/margin by active contract (bar chart)

**Personnel > Roster**
- DataTable with columns: Name, Role, Status, Current Site, Cert Status (badge), Phone
- Filters: Status, Role, Site, Cert Compliance
- Search by name
- "Add Crew Member" button → onboarding flow

**Personnel > Profile**
- Header: name, role, status badge, photo placeholder, quick actions (assign, message)
- Tabs: Details | Certifications | Equipment | Assignments | Documents | Drug Tests | Incidents
- Each tab is its own section with add/edit/upload capability

**Personnel > Compliance Matrix**
- Grid: rows = crew members, columns = cert types marked as required
- Cells: green (current), yellow (expiring < 90 days), red (expired/missing), gray (not required for this role)
- Click cell → cert record detail or "Add Cert" flow
- Bulk action: "Everyone on [Site] needs [Cert] by [Date]" → shows compliance status

**Equipment > Asset Table**
- DataTable: Asset Tag, Name, Category, Status, Location/Assigned To, Condition, Book Value
- Filters: Category, Ownership, Condition, Location
- Click → asset detail

**Equipment > Asset Detail**
- Header: photo placeholder, name, asset tag, QR code link, condition badge, location
- Tabs: Info | Maintenance | Fuel Log | Inspections | Chain of Custody
- Maintenance tab: timeline of work orders + "Schedule Maintenance" + "Report Issue" buttons
- Fuel log: table with running totals, cost-per-mile/hour calculations

**Equipment > Work Orders**
- Kanban: Reported → Assigned → In Progress → Complete
- Cards show: asset name, issue, priority, assigned to, age
- Drag to advance status

**Equipment > Fleet Dashboard**
- Utilization rates per category (% of fleet deployed)
- Top maintenance costs (bar chart, top 10 assets)
- Depreciation summary (total book value, monthly depreciation)
- Upcoming maintenance (calendar)

**Insurance > Policies**
- Shopping list section at top (amber highlight) — policies in "Shopping" status with quotes
- Active policies table: type, carrier, coverage, expiration, status badge
- Click → policy detail with payment history, linked contracts

**Insurance > Requirements Matrix**
- Grid: rows = active contracts, columns = required coverage types
- Cells: green (met, linked policy), red (gap), yellow (policy expiring soon)
- Click gap cell → "Link Policy" or "Add to Shopping List"

**Contracts > Pipeline**
- Board view: columns = status stages (Bidding through Closed Out)
- Cards: contract number, agency, value, performance period
- Click → contract detail

**Contracts > Detail**
- Header: contract/solicitation number, agency, CO name/contact, status, value
- Tabs: Overview | Sites | Financials | Insurance | Documents | Daily Reports
- Sites tab: map pins + list, click into site detail
- Financials: value, funded, invoiced, remaining, labor costs, equipment costs, margin

**Dispatch Board**
- Calendar (week view default, day/month available)
- Rows = crew members, columns = days
- Color-coded blocks = site assignments
- Drag to reassign or extend
- Conflict detection (crew double-booked highlights red)
- Quick assign panel: select crew → select site → set dates → confirm

**Documents**
- Upload form: drag-and-drop, select category, link to entity
- Table: filename, category, linked entity, uploaded by, date, version
- Full-text search bar
- "Ask" button → opens RAG query interface

**Documents > Ask (RAG)**
- Chat-style interface
- Scope selector: "All Documents" or specific contract/site
- AI answers with cited sources (document name, page/chunk reference)
- Rendered with AI Elements MessageResponse component

**Reports**
- Financial: revenue/cost/margin by contract, time period selector, exportable
- Compliance: cert matrix export (PDF/CSV), insurance coverage report
- Utilization: equipment hours/miles, maintenance cost per asset, fleet status

**Settings**
- Organization: company profile, divisions
- Users: invite supervisors (generates invite code), manage roles
- Cert Types: CRUD for certification type definitions
- Asset Categories: CRUD with inspection checklist templates
- Insurance Types: CRUD for policy type definitions
- Alerts: configure thresholds (30/60/90 day warnings), email settings

### Field View (Mobile — Supervisor)

**Layout:** Bottom tab navigation (4 tabs: My Site, Report, Inspect, Ask). Minimal header with site name and notification badge.

**My Site**
- Site name, status, scope summary (collapsible)
- Today's crew: list with name, role, phone (tap to call)
- Equipment on site: list with asset tag, name, condition
- Quick stats: days remaining on contract, hours logged this week

**Daily Report**
- Form (large touch targets):
  - Date (defaults to today)
  - Crew count (number input)
  - Total hours (number input)
  - Work accomplished (textarea)
  - Weather: conditions dropdown + temp
  - Safety notes (textarea)
  - Incident toggle → incident description field
  - Photo upload (camera or gallery, multiple)
  - Submit button (creates Draft, supervisor reviews, then Submits)

**Inspections**
- List of equipment assigned to this site
- Tap asset → inspection checklist (configured per asset category)
- Each item: Pass/Fail toggle + notes field
- Overall result auto-calculated
- Photo capture for issues
- Submit → saved to asset's inspection history

**Ask**
- Simple chat input
- Auto-scoped to supervisor's assigned site documents
- AI answers from contract SOW, wage determinations, safety plans
- Rendered with AI Elements MessageResponse

## Auth & Permissions

| Role | Command Center | Field View | Data Scope |
|------|---------------|------------|------------|
| Owner | Full access | Full access | Everything |
| Manager | Full access | Full access | Everything |
| Supervisor | Read-only | Full access | Assigned site(s) only |
| Crew Lead | No access | Read-only (My Site, Ask) | Assigned site only |
| Crew Member | No access | No access | N/A |

**Auth flow:**
1. Owner creates account during setup (email + password)
2. Owner invites supervisors: generates a unique invite code
3. Supervisor opens invite link → creates password → assigned role + site
4. All sessions use NextAuth.js with JWT strategy (no session DB needed)

## Notification System

**Triggers (daily cron scan):**
- Cert expiring within configured threshold (default: 30/60/90 days)
- Insurance policy expiring within threshold
- Equipment maintenance due (by hours, miles, or calendar date)
- Daily report not submitted by end of day for an active site
- Work order open > 7 days without progress

**Delivery:**
- In-app: notification bell with unread count, click to view/dismiss
- Email: via Resend for Critical and Warning severity

**Escalation:**
- 90 days: Info (amber badge)
- 60 days: Warning (email sent)
- 30 days: Critical (email sent, dashboard card turns red)

## RAG / Document Intelligence

**Ingestion pipeline:**
1. User uploads document, links to entity (contract, site, etc.)
2. `extract.ts` extracts text: PDF (pdf-parse), DOCX (mammoth), images (Tesseract OCR)
3. `ingest.ts` chunks text (512 tokens, 50 token overlap), generates embeddings via AI Gateway
4. Chunks stored in `DocumentChunk` table with pgvector embedding column
5. `embeddingStatus` on document updated to "Embedded"

**Query flow:**
1. User types question, optionally scopes to contract/site
2. `search.ts` generates query embedding, runs cosine similarity search with scope filter
3. Top-K chunks returned as context
4. AI Gateway LLM generates answer with source citations
5. Response streamed to client via AI SDK `streamText` + `useChat`

**Supervisor scoping:** field view queries automatically filter to documents linked to the supervisor's assigned site(s). No option to search outside their scope.

## Cron Jobs

| Job | Schedule | Purpose |
|-----|----------|---------|
| Alert scan | Daily 6:00 AM | Check cert/insurance/maintenance expirations, missing daily reports |
| Depreciation | Monthly 1st | Recalculate book values (optional, since depreciation is derived) |

## Non-Functional Requirements

- **Performance:** Server-rendered pages load < 2s on 3G for field view. Dashboard loads < 3s on broadband.
- **Security:** SSN encrypted at rest (AES-256-GCM). All routes auth-gated. RBAC enforced server-side. CSRF protection via Server Actions. Rate limiting on auth endpoints.
- **Availability:** Vercel's infrastructure provides automatic failover. Neon Postgres has built-in replication.
- **Backup:** Neon provides point-in-time recovery. Vercel Blob stores are durable. No manual backup system needed initially.
- **Scalability:** Neon auto-scales. Vercel Functions scale to demand. No user-managed infrastructure.
- **Accessibility:** shadcn/ui components are WCAG 2.1 AA compliant by default. Maintain keyboard navigation and screen reader support.

## Testing Strategy

### Unit Tests (Vitest)
- **Depreciation engine** — straight-line and MACRS calculations with edge cases (zero salvage, mid-year purchase)
- **Encryption helpers** — SSN encrypt/decrypt roundtrip, handling of invalid inputs
- **Permissions logic** — role hierarchy checks, site-scoped access, edge cases (no site assigned, multiple sites)
- **Alert scanner** — cert expiry detection at 30/60/90 day boundaries, insurance gaps, maintenance due calculations
- **Audit log writer** — correct old/new value capture, JSON serialization of complex fields

### Integration Tests (Vitest + Prisma test client)
- **Server Actions** — CRUD operations for each domain (personnel, equipment, contracts, etc.) with real DB
- **Auth flow** — login, invite code generation, invite acceptance, JWT validation, role-based route protection
- **RAG pipeline** — document upload → text extraction → chunking → embedding → search → scoped results
- **Notification creation** — cron trigger → expiration scan → notification records created with correct severity
- **Dispatch conflict detection** — double-booking detection, date range overlaps

### E2E Tests (Playwright)
- **Onboarding flow** — create crew member → add certs → assign to site → verify on dispatch board
- **Supervisor field flow** — login → view my site → submit daily report → submit inspection → ask RAG question
- **Equipment lifecycle** — add asset → check out to site → log fuel → report issue → work order → check in
- **Insurance workflow** — add shopping item → add quotes → purchase → link to contract → verify requirements matrix
- **Compliance matrix** — define cert types → set required for role → verify matrix colors → add cert → verify green

### Test Infrastructure
- Prisma test client with isolated test database (Neon branch per test suite)
- Factory functions for creating test data (personnel, assets, contracts, sites)
- Seed script for demo data (pre-populated crew, certs, equipment, contracts for dev/demo)

## Future Considerations (Not in Scope)

- SMS notifications (add when email isn't enough)
- Crew Member self-service portal (upload own certs, view schedule)
- GPS tracking of equipment in the field
- Payroll integration
- Invoice generation from daily reports
- Bid pipeline integration (bridge to existing bid-command system)
- Mobile offline mode (PWA with sync)
- Multi-org / multi-tenant (if Hoags expands to manage crews for other companies)
