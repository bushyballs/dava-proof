# Hoags Crew Command — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundation of Hoags Crew Command — project scaffold, database, auth, and the first two domain modules (Personnel + Equipment) — producing a working, testable web app.

**Architecture:** Next.js 16 App Router with Prisma + Neon Postgres, shadcn/ui dark theme, NextAuth.js for role-based access. Server Components by default, Server Actions for mutations. Audit trail on all writes.

**Tech Stack:** Next.js 16, TypeScript, Prisma, Neon Postgres, NextAuth.js, shadcn/ui, Tailwind CSS, Vitest, Playwright

**Spec:** `docs/superpowers/specs/2026-03-29-crew-command-design.md`

---

## File Map

### Foundation (Tasks 1-5)
| Action | Path | Purpose |
|--------|------|---------|
| Create | `hoags-crew-command/prisma/schema.prisma` | Full database schema |
| Create | `hoags-crew-command/src/lib/db.ts` | Prisma client singleton |
| Create | `hoags-crew-command/src/lib/auth.ts` | NextAuth config with credentials + JWT |
| Create | `hoags-crew-command/src/lib/permissions.ts` | Role hierarchy + site-scoped access checks |
| Create | `hoags-crew-command/src/lib/audit.ts` | Audit log writer wrapping Prisma mutations |
| Create | `hoags-crew-command/src/lib/encryption.ts` | AES-256-GCM encrypt/decrypt for SSN |
| Create | `hoags-crew-command/src/app/layout.tsx` | Root layout with dark theme + Geist fonts |
| Create | `hoags-crew-command/src/app/page.tsx` | Root redirect to /dashboard or /login |
| Create | `hoags-crew-command/src/app/(auth)/login/page.tsx` | Login page |
| Create | `hoags-crew-command/src/app/(auth)/invite/[code]/page.tsx` | Invite acceptance |
| Create | `hoags-crew-command/src/app/api/auth/[...nextauth]/route.ts` | NextAuth route handler |
| Create | `hoags-crew-command/src/app/(command)/layout.tsx` | Command center sidebar layout |
| Create | `hoags-crew-command/src/app/(command)/dashboard/page.tsx` | Dashboard placeholder |
| Create | `hoags-crew-command/src/components/ui/` | shadcn/ui components (via CLI) |
| Create | `hoags-crew-command/src/components/domain/stat-card.tsx` | Reusable stat card |
| Create | `hoags-crew-command/tests/lib/encryption.test.ts` | Encryption unit tests |
| Create | `hoags-crew-command/tests/lib/permissions.test.ts` | Permissions unit tests |
| Create | `hoags-crew-command/tests/lib/audit.test.ts` | Audit log unit tests |
| Create | `hoags-crew-command/tests/helpers/factories.ts` | Test data factory functions |

### Personnel Module (Tasks 6-8)
| Action | Path | Purpose |
|--------|------|---------|
| Create | `hoags-crew-command/src/actions/personnel.ts` | Server Actions for crew CRUD |
| Create | `hoags-crew-command/src/app/(command)/personnel/page.tsx` | Crew roster table |
| Create | `hoags-crew-command/src/app/(command)/personnel/[id]/page.tsx` | Crew profile with tabs |
| Create | `hoags-crew-command/src/app/(command)/personnel/new/page.tsx` | Onboarding form |
| Create | `hoags-crew-command/src/components/domain/crew-table.tsx` | DataTable for crew roster |
| Create | `hoags-crew-command/tests/actions/personnel.test.ts` | Personnel action tests |

### Training & Certs Module (Tasks 9-10)
| Action | Path | Purpose |
|--------|------|---------|
| Create | `hoags-crew-command/src/actions/training.ts` | Server Actions for certs + training |
| Create | `hoags-crew-command/src/app/(command)/training/cert-types/page.tsx` | Cert type admin |
| Create | `hoags-crew-command/src/app/(command)/personnel/compliance/page.tsx` | Compliance matrix |
| Create | `hoags-crew-command/src/components/domain/cert-badge.tsx` | Cert status badge |
| Create | `hoags-crew-command/src/components/domain/compliance-matrix.tsx` | Compliance grid |
| Create | `hoags-crew-command/tests/actions/training.test.ts` | Training action tests |

### Equipment Module (Tasks 11-14)
| Action | Path | Purpose |
|--------|------|---------|
| Create | `hoags-crew-command/src/actions/equipment.ts` | Server Actions for assets |
| Create | `hoags-crew-command/src/lib/depreciation.ts` | Depreciation calculations |
| Create | `hoags-crew-command/src/app/(command)/equipment/page.tsx` | Asset table |
| Create | `hoags-crew-command/src/app/(command)/equipment/[id]/page.tsx` | Asset detail with tabs |
| Create | `hoags-crew-command/src/app/(command)/equipment/new/page.tsx` | Add asset form |
| Create | `hoags-crew-command/src/app/(command)/equipment/categories/page.tsx` | Asset category admin |
| Create | `hoags-crew-command/src/components/domain/asset-row.tsx` | Asset table row |
| Create | `hoags-crew-command/tests/lib/depreciation.test.ts` | Depreciation unit tests |
| Create | `hoags-crew-command/tests/actions/equipment.test.ts` | Equipment action tests |

### Seed & E2E (Task 15)
| Action | Path | Purpose |
|--------|------|---------|
| Create | `hoags-crew-command/prisma/seed.ts` | Demo data seed script |
| Create | `hoags-crew-command/tests/e2e/personnel.spec.ts` | E2E: onboarding + certs flow |
| Create | `hoags-crew-command/tests/e2e/equipment.spec.ts` | E2E: asset lifecycle flow |

---

## Task 1: Project Scaffold

**Files:**
- Create: `hoags-crew-command/` (entire project root)

- [ ] **Step 1: Create the Next.js project**

```bash
cd C:/Users/colli/OneDrive/Desktop
npx create-next-app@latest hoags-crew-command --typescript --tailwind --eslint --app --src-dir --import-alias "@/*" --turbopack
```

When prompted for options, accept defaults (App Router, src/ directory, Turbopack).

- [ ] **Step 2: Install dependencies**

```bash
cd C:/Users/colli/OneDrive/Desktop/hoags-crew-command
npm install prisma @prisma/client next-auth@5 @auth/prisma-adapter bcryptjs
npm install -D @types/bcryptjs vitest @vitejs/plugin-react @testing-library/react @testing-library/jest-dom playwright @playwright/test
```

- [ ] **Step 3: Initialize Prisma**

```bash
npx prisma init --datasource-provider postgresql
```

This creates `prisma/schema.prisma` and `.env` with `DATABASE_URL`.

- [ ] **Step 4: Configure `.env.local`**

Create `hoags-crew-command/.env.local`:

```env
DATABASE_URL="postgresql://user:pass@your-neon-host/crew_command?sslmode=require"
NEXTAUTH_SECRET="generate-a-random-32-char-string-here"
NEXTAUTH_URL="http://localhost:3000"
ENCRYPTION_KEY="generate-a-random-32-byte-hex-string"
```

Note: Replace `DATABASE_URL` with actual Neon connection string once provisioned.

- [ ] **Step 5: Add Vitest config**

Create `hoags-crew-command/vitest.config.ts`:

```typescript
import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    globals: true,
    environment: "node",
    include: ["tests/**/*.test.ts"],
    exclude: ["tests/e2e/**"],
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
```

- [ ] **Step 6: Add Playwright config**

Create `hoags-crew-command/playwright.config.ts`:

```typescript
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  webServer: {
    command: "npm run dev",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
  },
  use: {
    baseURL: "http://localhost:3000",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "Mobile Chrome", use: { ...devices["Pixel 5"] } },
  ],
});
```

- [ ] **Step 7: Add test scripts to package.json**

In `hoags-crew-command/package.json`, add to `"scripts"`:

```json
"test": "vitest run",
"test:watch": "vitest",
"test:e2e": "playwright test",
"db:push": "prisma db push",
"db:seed": "tsx prisma/seed.ts",
"db:studio": "prisma studio"
```

- [ ] **Step 8: Initialize git and commit**

```bash
cd C:/Users/colli/OneDrive/Desktop/hoags-crew-command
git init
git add -A
git commit -m "feat: scaffold Next.js 16 project with Prisma, NextAuth, Vitest, Playwright"
```

---

## Task 2: Prisma Schema

**Files:**
- Create: `hoags-crew-command/prisma/schema.prisma`

- [ ] **Step 1: Write the complete Prisma schema**

Replace the contents of `hoags-crew-command/prisma/schema.prisma` with:

```prisma
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

// ──────────────────────────────────────────
// Auth & Organization
// ──────────────────────────────────────────

enum Role {
  OWNER
  MANAGER
  SUPERVISOR
  CREW_LEAD
  CREW_MEMBER
}

model User {
  id             String   @id @default(cuid())
  email          String   @unique
  hashedPassword String
  role           Role     @default(CREW_MEMBER)
  personnelId    String?  @unique
  personnel      Personnel? @relation(fields: [personnelId], references: [id])
  createdAt      DateTime @default(now())
  updatedAt      DateTime @updatedAt
}

model InviteCode {
  id        String   @id @default(cuid())
  code      String   @unique
  role      Role
  siteId    String?
  usedBy    String?
  usedAt    DateTime?
  createdAt DateTime @default(now())
  expiresAt DateTime
}

model Organization {
  id        String   @id @default(cuid())
  name      String
  legalName String?
  cageCode  String?
  uei       String?
  samStatus String?
  phone     String?
  email     String?
  address   String?
  createdAt DateTime @default(now())
  updatedAt DateTime @updatedAt
}

model Division {
  id             String   @id @default(cuid())
  organizationId String
  name           String
  description    String?
  contracts      Contract[]
  createdAt      DateTime @default(now())
  updatedAt      DateTime @updatedAt
}

// ──────────────────────────────────────────
// Personnel
// ──────────────────────────────────────────

enum PersonnelStatus {
  APPLICANT
  ONBOARDING
  ACTIVE
  ON_LEAVE
  TERMINATED
}

enum PayType {
  HOURLY
  SALARY
}

model Personnel {
  id                String          @id @default(cuid())
  firstName         String
  lastName          String
  ssnEncrypted      String?
  dob               DateTime?
  address           String?
  phone             String?
  email             String?
  status            PersonnelStatus @default(APPLICANT)
  role              Role            @default(CREW_MEMBER)
  hireDate          DateTime?
  terminationDate   DateTime?
  terminationReason String?
  payRate           Decimal?
  payType           PayType?
  overtimeRules     String?
  scaWageDetermId   String?
  photoUrl          String?

  user              User?
  emergencyContacts EmergencyContact[]
  documents         PersonnelDocument[]
  drugTests         DrugTest[]
  physicalFitness   PhysicalFitness[]
  medicalClearances MedicalClearance[]
  backgroundChecks  BackgroundCheck[]
  availability      AvailabilityException[]
  performanceNotes  PerformanceNote[]
  incidentReports   IncidentReport[]
  certRecords       CertRecord[]
  crewAssignments   CrewAssignment[]
  chainOfCustody    ChainOfCustody[]
  inspections       Inspection[]

  createdAt DateTime @default(now())
  updatedAt DateTime @updatedAt
  createdBy String?
  updatedBy String?
}

model EmergencyContact {
  id           String    @id @default(cuid())
  personnelId  String
  personnel    Personnel @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  name         String
  phone        String
  relationship String?
}

model PersonnelDocument {
  id          String    @id @default(cuid())
  personnelId String
  personnel   Personnel @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  category    String
  fileName    String
  blobUrl     String
  version     Int       @default(1)
  uploadedAt  DateTime  @default(now())
}

enum DrugTestType {
  PRE_EMPLOYMENT
  RANDOM
  POST_INCIDENT
  REASONABLE_SUSPICION
}

model DrugTest {
  id               String       @id @default(cuid())
  personnelId      String
  personnel        Personnel    @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  date             DateTime
  type             DrugTestType
  result           String
  lab              String?
  chainOfCustodyDoc String?
}

model PhysicalFitness {
  id          String    @id @default(cuid())
  personnelId String
  personnel   Personnel @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  date        DateTime
  type        String
  result      String
  provider    String?
  documentUrl String?
}

model MedicalClearance {
  id           String    @id @default(cuid())
  personnelId  String
  personnel    Personnel @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  date         DateTime
  type         String
  restrictions String?
  expiry       DateTime?
  documentUrl  String?
}

enum BackgroundCheckStatus {
  PENDING
  CLEARED
  FAILED
}

model BackgroundCheck {
  id          String                @id @default(cuid())
  personnelId String
  personnel   Personnel             @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  date        DateTime
  status      BackgroundCheckStatus
  provider    String?
  documentUrl String?
}

model AvailabilityException {
  id          String    @id @default(cuid())
  personnelId String
  personnel   Personnel @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  startDate   DateTime
  endDate     DateTime
  reason      String
}

model PerformanceNote {
  id          String    @id @default(cuid())
  personnelId String
  personnel   Personnel @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  date        DateTime  @default(now())
  author      String
  category    String?
  content     String
}

model IncidentReport {
  id               String    @id @default(cuid())
  personnelId      String
  personnel        Personnel @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  date             DateTime
  siteId           String?
  description      String
  severity         String
  witnesses        String?
  correctiveAction String?
  documentUrl      String?
}

// ──────────────────────────────────────────
// Certifications & Training
// ──────────────────────────────────────────

model CertType {
  id                      String       @id @default(cuid())
  name                    String
  category                String?
  description             String?
  expires                 Boolean      @default(true)
  validityMonths          Int?
  requiredForRoles        Role[]
  requiredForContractTypes String[]
  renewalRequirements     String?
  isActive                Boolean      @default(true)
  certRecords             CertRecord[]
  trainingEvents          TrainingEvent[]
  createdAt               DateTime     @default(now())
  updatedAt               DateTime     @updatedAt
}

enum CertStatus {
  CURRENT
  EXPIRED
  PENDING
  REVOKED
}

model CertRecord {
  id              String     @id @default(cuid())
  personnelId     String
  personnel       Personnel  @relation(fields: [personnelId], references: [id], onDelete: Cascade)
  certTypeId      String
  certType        CertType   @relation(fields: [certTypeId], references: [id])
  dateEarned      DateTime
  expirationDate  DateTime?
  status          CertStatus @default(CURRENT)
  issuingAuthority String?
  certNumber      String?
  documentUrl     String?
  notes           String?
  createdAt       DateTime   @default(now())
  updatedAt       DateTime   @updatedAt
}

model TrainingEvent {
  id          String    @id @default(cuid())
  certTypeId  String?
  certType    CertType? @relation(fields: [certTypeId], references: [id])
  date        DateTime
  endDate     DateTime?
  location    String?
  instructor  String?
  attendeeIds String[]
  hours       Decimal?
  topic       String
  description String?
  documentUrl String?
  createdAt   DateTime  @default(now())
}

// ──────────────────────────────────────────
// Equipment & Assets
// ──────────────────────────────────────────

model AssetCategory {
  id                        String   @id @default(cuid())
  name                      String
  description               String?
  inspectionChecklistTemplate Json?
  assets                    Asset[]
  createdAt                 DateTime @default(now())
  updatedAt                 DateTime @updatedAt
}

enum Ownership {
  OWNED
  RENTED
  LEASED
}

enum AssetCondition {
  EXCELLENT
  GOOD
  FAIR
  POOR
  OUT_OF_SERVICE
}

enum AssetLocation {
  IN_STORAGE
  IN_SHOP
  ASSIGNED_TO_SITE
  ASSIGNED_TO_PERSON
}

enum DepreciationMethod {
  STRAIGHT_LINE
  MACRS
}

model Asset {
  id                 String             @id @default(cuid())
  assetTag           String             @unique
  categoryId         String
  category           AssetCategory      @relation(fields: [categoryId], references: [id])
  name               String
  make               String?
  model              String?
  year               Int?
  serialNumber       String?
  vin                String?
  purchaseDate       DateTime?
  purchasePrice      Decimal?
  vendor             String?
  warrantyExpiry     DateTime?
  depreciationMethod DepreciationMethod?
  usefulLifeMonths   Int?
  salvageValue       Decimal?
  ownership          Ownership          @default(OWNED)
  rentalSource       String?
  rentalRate         Decimal?
  rentalPeriod       String?
  condition          AssetCondition     @default(GOOD)
  location           AssetLocation      @default(IN_STORAGE)
  assignedSiteId     String?
  assignedPersonnelId String?
  retiredDate        DateTime?
  retiredReason      String?
  disposalMethod     String?
  disposalValue      Decimal?
  photoUrl           String?

  workOrders     WorkOrder[]
  fuelLogs       FuelLog[]
  inspections    Inspection[]
  chainOfCustody ChainOfCustody[]
  siteAssignments EquipmentAssignment[]

  createdAt DateTime @default(now())
  updatedAt DateTime @updatedAt
  createdBy String?
  updatedBy String?
}

enum WorkOrderType {
  SCHEDULED
  REPORTED_ISSUE
  RECALL
}

enum WorkOrderStatus {
  REPORTED
  ASSIGNED
  IN_PROGRESS
  COMPLETE
  CANCELLED
}

model WorkOrder {
  id                String          @id @default(cuid())
  assetId           String
  asset             Asset           @relation(fields: [assetId], references: [id])
  type              WorkOrderType   @default(REPORTED_ISSUE)
  status            WorkOrderStatus @default(REPORTED)
  reportedBy        String?
  assignedTo        String?
  reportedDate      DateTime        @default(now())
  completedDate     DateTime?
  description       String
  resolution        String?
  parts             Json?
  laborHours        Decimal?
  totalCost         Decimal?
  nextScheduledDate DateTime?
  scheduledBy       String?
}

model FuelLog {
  id            String   @id @default(cuid())
  assetId       String
  asset         Asset    @relation(fields: [assetId], references: [id])
  date          DateTime
  gallons       Decimal
  costPerGallon Decimal?
  totalCost     Decimal?
  odometer      Int?
  engineHours   Int?
  location      String?
  receiptUrl    String?
  createdAt     DateTime @default(now())
}

enum InspectionResult {
  PASS
  FAIL
  CONDITIONAL
}

model Inspection {
  id             String           @id @default(cuid())
  assetId        String
  asset          Asset            @relation(fields: [assetId], references: [id])
  personnelId    String
  personnel      Personnel        @relation(fields: [personnelId], references: [id])
  date           DateTime         @default(now())
  checklistItems Json
  overallResult  InspectionResult
  notes          String?
  photoUrls      String[]
}

model ChainOfCustody {
  id                  String    @id @default(cuid())
  assetId             String
  asset               Asset     @relation(fields: [assetId], references: [id])
  personnelId         String
  personnel           Personnel @relation(fields: [personnelId], references: [id])
  checkedOutDate      DateTime  @default(now())
  checkedInDate       DateTime?
  conditionAtCheckout String?
  conditionAtCheckin  String?
  siteId              String?
  notes               String?
}

// ──────────────────────────────────────────
// Contracts & Sites
// ──────────────────────────────────────────

enum ContractType {
  FIRM_FIXED
  TIME_AND_MATERIALS
  IDIQ
  BPA
}

enum ContractStatus {
  BIDDING
  AWARDED
  MOBILIZING
  ACTIVE
  DEMOBILIZING
  COMPLETE
  CLOSED_OUT
}

model Contract {
  id                    String         @id @default(cuid())
  solicitationNumber    String?
  contractNumber        String?
  agency                String?
  coName                String?
  coEmail               String?
  coPhone               String?
  type                  ContractType?
  performancePeriodStart DateTime?
  performancePeriodEnd  DateTime?
  optionYears           Json?
  contractValue         Decimal?
  fundedAmount          Decimal?
  invoicedToDate        Decimal?       @default(0)
  status                ContractStatus @default(BIDDING)
  divisionId            String?
  division              Division?      @relation(fields: [divisionId], references: [id])
  notes                 String?
  sites                 Site[]
  insuranceRequirements InsuranceRequirement[]
  createdAt             DateTime       @default(now())
  updatedAt             DateTime       @updatedAt
  createdBy             String?
  updatedBy             String?
}

enum SiteStatus {
  PLANNED
  ACTIVE
  DEMOBILIZING
  COMPLETE
}

model Site {
  id               String     @id @default(cuid())
  contractId       String
  contract         Contract   @relation(fields: [contractId], references: [id])
  name             String
  address          String?
  gpsLat           Decimal?
  gpsLon           Decimal?
  acreage          Decimal?
  accessNotes      String?
  hazardNotes      String?
  scopeDescription String?
  status           SiteStatus @default(PLANNED)
  crewAssignments  CrewAssignment[]
  equipmentAssignments EquipmentAssignment[]
  dailyReports     DailyReport[]
  createdAt        DateTime   @default(now())
  updatedAt        DateTime   @updatedAt
}

enum AssignmentStatus {
  SCHEDULED
  ACTIVE
  COMPLETE
}

model CrewAssignment {
  id          String           @id @default(cuid())
  personnelId String
  personnel   Personnel        @relation(fields: [personnelId], references: [id])
  siteId      String
  site        Site             @relation(fields: [siteId], references: [id])
  role        String?
  startDate   DateTime
  endDate     DateTime?
  status      AssignmentStatus @default(SCHEDULED)
}

model EquipmentAssignment {
  id           String    @id @default(cuid())
  assetId      String
  asset        Asset     @relation(fields: [assetId], references: [id])
  siteId       String
  site         Site      @relation(fields: [siteId], references: [id])
  deployedDate DateTime  @default(now())
  returnedDate DateTime?
  notes        String?
}

enum DailyReportStatus {
  DRAFT
  SUBMITTED
  REVIEWED
}

model DailyReport {
  id                  String            @id @default(cuid())
  siteId              String
  site                Site              @relation(fields: [siteId], references: [id])
  submittedBy         String
  date                DateTime
  crewCount           Int
  totalHours          Decimal
  workAccomplished    String
  weather             String?
  safetyNotes         String?
  incidentOccurred    Boolean           @default(false)
  incidentDescription String?
  photoUrls           String[]
  status              DailyReportStatus @default(DRAFT)
  createdAt           DateTime          @default(now())
  updatedAt           DateTime          @updatedAt
}

// ──────────────────────────────────────────
// Insurance
// ──────────────────────────────────────────

enum InsuranceStatus {
  ACTIVE
  EXPIRED
  SHOPPING
  CANCELLED
}

model InsurancePolicy {
  id                   String          @id @default(cuid())
  type                 String
  status               InsuranceStatus @default(SHOPPING)
  carrier              String?
  agentName            String?
  agentPhone           String?
  agentEmail           String?
  policyNumber         String?
  coveragePerOccurrence Decimal?
  coverageAggregate    Decimal?
  premium              Decimal?
  paymentSchedule      String?
  effectiveDate        DateTime?
  expirationDate       DateTime?
  documentUrl          String?
  notes                String?
  quotes               InsuranceQuote[]
  coiCertificates      CoiCertificate[]
  requirements         InsuranceRequirement[]
  createdAt            DateTime        @default(now())
  updatedAt            DateTime        @updatedAt
}

model InsuranceQuote {
  id         String          @id @default(cuid())
  policyId   String
  policy     InsurancePolicy @relation(fields: [policyId], references: [id])
  carrier    String
  premium    Decimal?
  coverage   String?
  quoteDate  DateTime
  documentUrl String?
}

model CoiCertificate {
  id             String          @id @default(cuid())
  policyId       String
  policy         InsurancePolicy @relation(fields: [policyId], references: [id])
  contractId     String?
  recipientName  String
  recipientEmail String?
  sentDate       DateTime?
  documentUrl    String?
  notes          String?
}

enum RequirementStatus {
  MET
  GAP
  PENDING
}

model InsuranceRequirement {
  id                String            @id @default(cuid())
  contractId        String
  contract          Contract          @relation(fields: [contractId], references: [id])
  requiredPolicyType String
  minimumCoverage   Decimal?
  linkedPolicyId    String?
  linkedPolicy      InsurancePolicy?  @relation(fields: [linkedPolicyId], references: [id])
  status            RequirementStatus @default(GAP)
}

// ──────────────────────────────────────────
// Notifications & Audit
// ──────────────────────────────────────────

enum NotificationType {
  CERT_EXPIRING
  INSURANCE_EXPIRING
  MAINTENANCE_DUE
  DAILY_REPORT_MISSING
  CONTRACT_AWARD
}

enum Severity {
  INFO
  WARNING
  CRITICAL
}

model Notification {
  id               String           @id @default(cuid())
  type             NotificationType
  severity         Severity         @default(INFO)
  title            String
  message          String
  linkedEntityType String?
  linkedEntityId   String?
  recipientId      String?
  read             Boolean          @default(false)
  createdAt        DateTime         @default(now())
}

enum AuditAction {
  CREATE
  UPDATE
  DELETE
}

model AuditLog {
  id        String      @id @default(cuid())
  tableName String
  recordId  String
  action    AuditAction
  changedBy String
  changedAt DateTime    @default(now())
  oldValues Json?
  newValues Json?
}
```

- [ ] **Step 2: Push schema to database**

```bash
cd C:/Users/colli/OneDrive/Desktop/hoags-crew-command
npx prisma db push
```

Expected: Schema synced to Neon. If no `DATABASE_URL` is set yet, this step will fail — that's OK, it can be run once the Neon instance is provisioned.

- [ ] **Step 3: Generate Prisma client**

```bash
npx prisma generate
```

Expected: Prisma Client generated to `node_modules/.prisma/client`.

- [ ] **Step 4: Commit**

```bash
git add prisma/schema.prisma
git commit -m "feat: add complete Prisma schema — personnel, certs, equipment, contracts, insurance, audit"
```

---

## Task 3: Core Library — DB, Encryption, Permissions, Audit

**Files:**
- Create: `hoags-crew-command/src/lib/db.ts`
- Create: `hoags-crew-command/src/lib/encryption.ts`
- Create: `hoags-crew-command/src/lib/permissions.ts`
- Create: `hoags-crew-command/src/lib/audit.ts`
- Create: `hoags-crew-command/tests/lib/encryption.test.ts`
- Create: `hoags-crew-command/tests/lib/permissions.test.ts`

- [ ] **Step 1: Write encryption tests**

Create `hoags-crew-command/tests/lib/encryption.test.ts`:

```typescript
import { describe, it, expect, beforeAll } from "vitest";

// We'll set a test key before importing
process.env.ENCRYPTION_KEY = "a]b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3";

import { encrypt, decrypt } from "@/lib/encryption";

describe("encryption", () => {
  it("encrypts and decrypts a string roundtrip", () => {
    const plaintext = "123-45-6789";
    const encrypted = encrypt(plaintext);
    expect(encrypted).not.toBe(plaintext);
    expect(encrypted).toContain(":");
    const decrypted = decrypt(encrypted);
    expect(decrypted).toBe(plaintext);
  });

  it("produces different ciphertext for the same input (random IV)", () => {
    const plaintext = "123-45-6789";
    const a = encrypt(plaintext);
    const b = encrypt(plaintext);
    expect(a).not.toBe(b);
  });

  it("throws on empty input", () => {
    expect(() => encrypt("")).toThrow();
  });

  it("throws on tampered ciphertext", () => {
    const encrypted = encrypt("123-45-6789");
    const tampered = encrypted.slice(0, -2) + "00";
    expect(() => decrypt(tampered)).toThrow();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd C:/Users/colli/OneDrive/Desktop/hoags-crew-command
npx vitest run tests/lib/encryption.test.ts
```

Expected: FAIL — `Cannot find module '@/lib/encryption'`

- [ ] **Step 3: Implement encryption**

Create `hoags-crew-command/src/lib/encryption.ts`:

```typescript
import { createCipheriv, createDecipheriv, randomBytes } from "crypto";

const ALGORITHM = "aes-256-gcm";
const IV_LENGTH = 16;
const AUTH_TAG_LENGTH = 16;

function getKey(): Buffer {
  const hex = process.env.ENCRYPTION_KEY;
  if (!hex || hex.length !== 64) {
    throw new Error("ENCRYPTION_KEY must be 64 hex characters (32 bytes)");
  }
  return Buffer.from(hex, "hex");
}

export function encrypt(plaintext: string): string {
  if (!plaintext) throw new Error("Cannot encrypt empty string");
  const key = getKey();
  const iv = randomBytes(IV_LENGTH);
  const cipher = createCipheriv(ALGORITHM, key, iv, { authTagLength: AUTH_TAG_LENGTH });
  const encrypted = Buffer.concat([cipher.update(plaintext, "utf8"), cipher.final()]);
  const authTag = cipher.getAuthTag();
  return `${iv.toString("hex")}:${authTag.toString("hex")}:${encrypted.toString("hex")}`;
}

export function decrypt(ciphertext: string): string {
  const key = getKey();
  const [ivHex, authTagHex, encryptedHex] = ciphertext.split(":");
  if (!ivHex || !authTagHex || !encryptedHex) {
    throw new Error("Invalid ciphertext format");
  }
  const iv = Buffer.from(ivHex, "hex");
  const authTag = Buffer.from(authTagHex, "hex");
  const encrypted = Buffer.from(encryptedHex, "hex");
  const decipher = createDecipheriv(ALGORITHM, key, iv, { authTagLength: AUTH_TAG_LENGTH });
  decipher.setAuthTag(authTag);
  return decipher.update(encrypted) + decipher.final("utf8");
}
```

- [ ] **Step 4: Run encryption tests**

```bash
npx vitest run tests/lib/encryption.test.ts
```

Expected: 4 tests PASS

- [ ] **Step 5: Write permissions tests**

Create `hoags-crew-command/tests/lib/permissions.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { canAccess, isAtLeast, ROLE_HIERARCHY } from "@/lib/permissions";

describe("permissions", () => {
  it("OWNER outranks all other roles", () => {
    expect(isAtLeast("OWNER", "OWNER")).toBe(true);
    expect(isAtLeast("OWNER", "MANAGER")).toBe(true);
    expect(isAtLeast("OWNER", "CREW_MEMBER")).toBe(true);
  });

  it("CREW_MEMBER cannot access MANAGER level", () => {
    expect(isAtLeast("CREW_MEMBER", "MANAGER")).toBe(false);
  });

  it("SUPERVISOR can access their assigned site", () => {
    const result = canAccess({
      userRole: "SUPERVISOR",
      requiredRole: "SUPERVISOR",
      userSiteIds: ["site-1", "site-2"],
      targetSiteId: "site-1",
    });
    expect(result).toBe(true);
  });

  it("SUPERVISOR cannot access unassigned site", () => {
    const result = canAccess({
      userRole: "SUPERVISOR",
      requiredRole: "SUPERVISOR",
      userSiteIds: ["site-1"],
      targetSiteId: "site-99",
    });
    expect(result).toBe(false);
  });

  it("OWNER can access any site regardless of assignment", () => {
    const result = canAccess({
      userRole: "OWNER",
      requiredRole: "SUPERVISOR",
      userSiteIds: [],
      targetSiteId: "site-99",
    });
    expect(result).toBe(true);
  });

  it("role hierarchy has correct ordering", () => {
    expect(ROLE_HIERARCHY.OWNER).toBeGreaterThan(ROLE_HIERARCHY.MANAGER);
    expect(ROLE_HIERARCHY.MANAGER).toBeGreaterThan(ROLE_HIERARCHY.SUPERVISOR);
    expect(ROLE_HIERARCHY.SUPERVISOR).toBeGreaterThan(ROLE_HIERARCHY.CREW_LEAD);
    expect(ROLE_HIERARCHY.CREW_LEAD).toBeGreaterThan(ROLE_HIERARCHY.CREW_MEMBER);
  });
});
```

- [ ] **Step 6: Run test to verify it fails**

```bash
npx vitest run tests/lib/permissions.test.ts
```

Expected: FAIL — `Cannot find module '@/lib/permissions'`

- [ ] **Step 7: Implement permissions**

Create `hoags-crew-command/src/lib/permissions.ts`:

```typescript
import type { Role } from "@prisma/client";

export const ROLE_HIERARCHY: Record<Role, number> = {
  OWNER: 50,
  MANAGER: 40,
  SUPERVISOR: 30,
  CREW_LEAD: 20,
  CREW_MEMBER: 10,
};

export function isAtLeast(userRole: Role | string, requiredRole: Role | string): boolean {
  const userLevel = ROLE_HIERARCHY[userRole as Role] ?? 0;
  const requiredLevel = ROLE_HIERARCHY[requiredRole as Role] ?? 0;
  return userLevel >= requiredLevel;
}

export function canAccess(params: {
  userRole: Role | string;
  requiredRole: Role | string;
  userSiteIds?: string[];
  targetSiteId?: string;
}): boolean {
  if (!isAtLeast(params.userRole, params.requiredRole)) {
    return false;
  }
  // OWNER and MANAGER see everything — no site scoping
  if (isAtLeast(params.userRole, "MANAGER")) {
    return true;
  }
  // SUPERVISOR and below are site-scoped
  if (params.targetSiteId && params.userSiteIds) {
    return params.userSiteIds.includes(params.targetSiteId);
  }
  // No site context required — allow
  return true;
}
```

- [ ] **Step 8: Run permissions tests**

```bash
npx vitest run tests/lib/permissions.test.ts
```

Expected: 6 tests PASS

- [ ] **Step 9: Implement Prisma client singleton**

Create `hoags-crew-command/src/lib/db.ts`:

```typescript
import { PrismaClient } from "@prisma/client";

const globalForPrisma = globalThis as unknown as { prisma: PrismaClient };

export const db = globalForPrisma.prisma || new PrismaClient();

if (process.env.NODE_ENV !== "production") {
  globalForPrisma.prisma = db;
}
```

- [ ] **Step 10: Implement audit log writer**

Create `hoags-crew-command/src/lib/audit.ts`:

```typescript
import { db } from "./db";
import type { AuditAction } from "@prisma/client";

export async function writeAuditLog(params: {
  tableName: string;
  recordId: string;
  action: AuditAction;
  changedBy: string;
  oldValues?: Record<string, unknown> | null;
  newValues?: Record<string, unknown> | null;
}) {
  await db.auditLog.create({
    data: {
      tableName: params.tableName,
      recordId: params.recordId,
      action: params.action,
      changedBy: params.changedBy,
      oldValues: params.oldValues ?? undefined,
      newValues: params.newValues ?? undefined,
    },
  });
}

export async function auditedCreate<T extends { id: string }>(
  tableName: string,
  changedBy: string,
  createFn: () => Promise<T>,
): Promise<T> {
  const record = await createFn();
  await writeAuditLog({
    tableName,
    recordId: record.id,
    action: "CREATE",
    changedBy,
    newValues: record as unknown as Record<string, unknown>,
  });
  return record;
}

export async function auditedUpdate<T extends { id: string }>(
  tableName: string,
  changedBy: string,
  oldRecord: Record<string, unknown>,
  updateFn: () => Promise<T>,
): Promise<T> {
  const record = await updateFn();
  await writeAuditLog({
    tableName,
    recordId: record.id,
    action: "UPDATE",
    changedBy,
    oldValues: oldRecord,
    newValues: record as unknown as Record<string, unknown>,
  });
  return record;
}

export async function auditedDelete(
  tableName: string,
  recordId: string,
  changedBy: string,
  oldRecord: Record<string, unknown>,
): Promise<void> {
  await writeAuditLog({
    tableName,
    recordId,
    action: "DELETE",
    changedBy,
    oldValues: oldRecord,
  });
}
```

- [ ] **Step 11: Commit**

```bash
git add src/lib/ tests/lib/
git commit -m "feat: add core lib — encryption, permissions, audit, Prisma client"
```

---

## Task 4: Auth — NextAuth + Login + Invite

**Files:**
- Create: `hoags-crew-command/src/lib/auth.ts`
- Create: `hoags-crew-command/src/app/api/auth/[...nextauth]/route.ts`
- Create: `hoags-crew-command/src/app/(auth)/login/page.tsx`
- Create: `hoags-crew-command/src/app/(auth)/invite/[code]/page.tsx`

- [ ] **Step 1: Implement NextAuth config**

Create `hoags-crew-command/src/lib/auth.ts`:

```typescript
import NextAuth from "next-auth";
import CredentialsProvider from "next-auth/providers/credentials";
import { compare } from "bcryptjs";
import { db } from "./db";

export const { handlers, signIn, signOut, auth } = NextAuth({
  session: { strategy: "jwt" },
  pages: {
    signIn: "/login",
  },
  providers: [
    CredentialsProvider({
      name: "credentials",
      credentials: {
        email: { label: "Email", type: "email" },
        password: { label: "Password", type: "password" },
      },
      async authorize(credentials) {
        if (!credentials?.email || !credentials?.password) return null;
        const user = await db.user.findUnique({
          where: { email: credentials.email as string },
        });
        if (!user) return null;
        const valid = await compare(credentials.password as string, user.hashedPassword);
        if (!valid) return null;
        return {
          id: user.id,
          email: user.email,
          role: user.role,
          personnelId: user.personnelId,
        };
      },
    }),
  ],
  callbacks: {
    async jwt({ token, user }) {
      if (user) {
        token.role = (user as { role: string }).role;
        token.personnelId = (user as { personnelId: string | null }).personnelId;
      }
      return token;
    },
    async session({ session, token }) {
      if (session.user) {
        session.user.id = token.sub!;
        (session.user as { role: string }).role = token.role as string;
        (session.user as { personnelId: string | null }).personnelId = token.personnelId as string | null;
      }
      return session;
    },
  },
});
```

- [ ] **Step 2: Create NextAuth route handler**

Create `hoags-crew-command/src/app/api/auth/[...nextauth]/route.ts`:

```typescript
import { handlers } from "@/lib/auth";

export const { GET, POST } = handlers;
```

- [ ] **Step 3: Create login page**

Create `hoags-crew-command/src/app/(auth)/login/page.tsx`:

```tsx
import { redirect } from "next/navigation";
import { auth } from "@/lib/auth";
import { LoginForm } from "./login-form";

export default async function LoginPage() {
  const session = await auth();
  if (session?.user) redirect("/dashboard");
  return (
    <div className="flex min-h-screen items-center justify-center bg-zinc-950">
      <div className="w-full max-w-sm space-y-6 rounded-lg border border-zinc-800 bg-zinc-900 p-8">
        <div className="space-y-2 text-center">
          <h1 className="text-2xl font-bold text-zinc-100">Crew Command</h1>
          <p className="text-sm text-zinc-400">Sign in to your account</p>
        </div>
        <LoginForm />
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Create login form (client component)**

Create `hoags-crew-command/src/app/(auth)/login/login-form.tsx`:

```tsx
"use client";

import { signIn } from "next-auth/react";
import { useRouter } from "next/navigation";
import { useState } from "react";

export function LoginForm() {
  const router = useRouter();
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setLoading(true);
    setError("");
    const formData = new FormData(e.currentTarget);
    const result = await signIn("credentials", {
      email: formData.get("email") as string,
      password: formData.get("password") as string,
      redirect: false,
    });
    setLoading(false);
    if (result?.error) {
      setError("Invalid email or password");
    } else {
      router.push("/dashboard");
    }
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="space-y-2">
        <label htmlFor="email" className="text-sm font-medium text-zinc-300">
          Email
        </label>
        <input
          id="email"
          name="email"
          type="email"
          required
          className="w-full rounded-md border border-zinc-700 bg-zinc-800 px-3 py-2 text-zinc-100 placeholder:text-zinc-500 focus:border-amber-500 focus:outline-none focus:ring-1 focus:ring-amber-500"
          placeholder="you@hoagsinc.com"
        />
      </div>
      <div className="space-y-2">
        <label htmlFor="password" className="text-sm font-medium text-zinc-300">
          Password
        </label>
        <input
          id="password"
          name="password"
          type="password"
          required
          className="w-full rounded-md border border-zinc-700 bg-zinc-800 px-3 py-2 text-zinc-100 placeholder:text-zinc-500 focus:border-amber-500 focus:outline-none focus:ring-1 focus:ring-amber-500"
        />
      </div>
      {error && <p className="text-sm text-red-400">{error}</p>}
      <button
        type="submit"
        disabled={loading}
        className="w-full rounded-md bg-amber-600 px-4 py-2 font-medium text-zinc-950 hover:bg-amber-500 disabled:opacity-50"
      >
        {loading ? "Signing in..." : "Sign In"}
      </button>
    </form>
  );
}
```

- [ ] **Step 5: Create invite acceptance page**

Create `hoags-crew-command/src/app/(auth)/invite/[code]/page.tsx`:

```tsx
import { db } from "@/lib/db";
import { notFound } from "next/navigation";
import { InviteForm } from "./invite-form";

export default async function InvitePage({ params }: { params: Promise<{ code: string }> }) {
  const { code } = await params;
  const invite = await db.inviteCode.findUnique({ where: { code } });
  if (!invite || invite.usedBy || invite.expiresAt < new Date()) {
    notFound();
  }
  return (
    <div className="flex min-h-screen items-center justify-center bg-zinc-950">
      <div className="w-full max-w-sm space-y-6 rounded-lg border border-zinc-800 bg-zinc-900 p-8">
        <div className="space-y-2 text-center">
          <h1 className="text-2xl font-bold text-zinc-100">Join Crew Command</h1>
          <p className="text-sm text-zinc-400">
            You&apos;ve been invited as <span className="font-mono text-amber-400">{invite.role}</span>
          </p>
        </div>
        <InviteForm code={code} role={invite.role} />
      </div>
    </div>
  );
}
```

- [ ] **Step 6: Create invite form (client component)**

Create `hoags-crew-command/src/app/(auth)/invite/[code]/invite-form.tsx`:

```tsx
"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

export function InviteForm({ code, role }: { code: string; role: string }) {
  const router = useRouter();
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setLoading(true);
    setError("");
    const formData = new FormData(e.currentTarget);
    const res = await fetch("/api/auth/accept-invite", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        code,
        email: formData.get("email"),
        password: formData.get("password"),
      }),
    });
    setLoading(false);
    if (!res.ok) {
      const data = await res.json();
      setError(data.error || "Failed to create account");
    } else {
      router.push("/login");
    }
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="space-y-2">
        <label htmlFor="email" className="text-sm font-medium text-zinc-300">Email</label>
        <input id="email" name="email" type="email" required
          className="w-full rounded-md border border-zinc-700 bg-zinc-800 px-3 py-2 text-zinc-100 placeholder:text-zinc-500 focus:border-amber-500 focus:outline-none focus:ring-1 focus:ring-amber-500"
        />
      </div>
      <div className="space-y-2">
        <label htmlFor="password" className="text-sm font-medium text-zinc-300">Password</label>
        <input id="password" name="password" type="password" required minLength={8}
          className="w-full rounded-md border border-zinc-700 bg-zinc-800 px-3 py-2 text-zinc-100 placeholder:text-zinc-500 focus:border-amber-500 focus:outline-none focus:ring-1 focus:ring-amber-500"
        />
      </div>
      {error && <p className="text-sm text-red-400">{error}</p>}
      <button type="submit" disabled={loading}
        className="w-full rounded-md bg-amber-600 px-4 py-2 font-medium text-zinc-950 hover:bg-amber-500 disabled:opacity-50"
      >
        {loading ? "Creating account..." : "Create Account"}
      </button>
    </form>
  );
}
```

- [ ] **Step 7: Create invite acceptance API route**

Create `hoags-crew-command/src/app/api/auth/accept-invite/route.ts`:

```typescript
import { NextResponse } from "next/server";
import { hash } from "bcryptjs";
import { db } from "@/lib/db";

export async function POST(req: Request) {
  const { code, email, password } = await req.json();
  if (!code || !email || !password) {
    return NextResponse.json({ error: "Missing fields" }, { status: 400 });
  }
  const invite = await db.inviteCode.findUnique({ where: { code } });
  if (!invite || invite.usedBy || invite.expiresAt < new Date()) {
    return NextResponse.json({ error: "Invalid or expired invite" }, { status: 400 });
  }
  const existing = await db.user.findUnique({ where: { email } });
  if (existing) {
    return NextResponse.json({ error: "Email already registered" }, { status: 400 });
  }
  const hashedPassword = await hash(password, 12);
  await db.$transaction([
    db.user.create({ data: { email, hashedPassword, role: invite.role } }),
    db.inviteCode.update({ where: { code }, data: { usedBy: email, usedAt: new Date() } }),
  ]);
  return NextResponse.json({ ok: true });
}
```

- [ ] **Step 8: Commit**

```bash
git add src/lib/auth.ts src/app/api/auth/ src/app/\(auth\)/
git commit -m "feat: add NextAuth credentials auth with login and invite flow"
```

---

## Task 5: Command Center Layout + Dashboard Shell

**Files:**
- Create: `hoags-crew-command/src/app/(command)/layout.tsx`
- Create: `hoags-crew-command/src/app/(command)/dashboard/page.tsx`
- Create: `hoags-crew-command/src/app/layout.tsx`
- Create: `hoags-crew-command/src/app/page.tsx`
- Create: `hoags-crew-command/src/components/domain/stat-card.tsx`

- [ ] **Step 1: Install shadcn/ui**

```bash
cd C:/Users/colli/OneDrive/Desktop/hoags-crew-command
npx shadcn@latest init -d
```

Accept defaults (New York style, zinc, CSS variables).

- [ ] **Step 2: Add shadcn components needed for Phase 1**

```bash
npx shadcn@latest add button card table badge tabs input label select textarea dialog dropdown-menu separator sheet avatar tooltip
```

- [ ] **Step 3: Create root layout**

Replace `hoags-crew-command/src/app/layout.tsx`:

```tsx
import type { Metadata } from "next";
import { GeistSans } from "geist/font/sans";
import { GeistMono } from "geist/font/mono";
import "./globals.css";

export const metadata: Metadata = {
  title: "Hoags Crew Command",
  description: "Enterprise crew operations management",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${GeistSans.variable} ${GeistMono.variable} font-sans antialiased bg-zinc-950 text-zinc-100`}>
        {children}
      </body>
    </html>
  );
}
```

- [ ] **Step 4: Install Geist fonts**

```bash
npm install geist
```

- [ ] **Step 5: Create root page redirect**

Replace `hoags-crew-command/src/app/page.tsx`:

```tsx
import { redirect } from "next/navigation";
import { auth } from "@/lib/auth";

export default async function RootPage() {
  const session = await auth();
  if (session?.user) {
    redirect("/dashboard");
  }
  redirect("/login");
}
```

- [ ] **Step 6: Create command center sidebar layout**

Create `hoags-crew-command/src/app/(command)/layout.tsx`:

```tsx
import { auth } from "@/lib/auth";
import { redirect } from "next/navigation";
import { isAtLeast } from "@/lib/permissions";
import Link from "next/link";
import {
  LayoutDashboard,
  Users,
  Award,
  Wrench,
  Shield,
  FileText,
  Calendar,
  FolderOpen,
  BarChart3,
  Settings,
} from "lucide-react";

const NAV_ITEMS = [
  { href: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
  { href: "/personnel", label: "Personnel", icon: Users },
  { href: "/training/cert-types", label: "Training", icon: Award },
  { href: "/equipment", label: "Equipment", icon: Wrench },
  { href: "/insurance", label: "Insurance", icon: Shield },
  { href: "/contracts", label: "Contracts", icon: FileText },
  { href: "/dispatch", label: "Dispatch", icon: Calendar },
  { href: "/documents", label: "Documents", icon: FolderOpen },
  { href: "/reports", label: "Reports", icon: BarChart3 },
  { href: "/settings", label: "Settings", icon: Settings },
];

export default async function CommandLayout({ children }: { children: React.ReactNode }) {
  const session = await auth();
  if (!session?.user) redirect("/login");
  const role = (session.user as { role: string }).role;
  if (!isAtLeast(role, "SUPERVISOR")) redirect("/login");

  return (
    <div className="flex h-screen">
      <aside className="flex w-56 flex-col border-r border-zinc-800 bg-zinc-900">
        <div className="flex h-14 items-center border-b border-zinc-800 px-4">
          <span className="text-lg font-bold text-amber-500">Crew Command</span>
        </div>
        <nav className="flex-1 space-y-1 p-2">
          {NAV_ITEMS.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className="flex items-center gap-3 rounded-md px-3 py-2 text-sm text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </Link>
          ))}
        </nav>
        <div className="border-t border-zinc-800 p-4">
          <p className="truncate text-xs text-zinc-500">{session.user.email}</p>
          <p className="font-mono text-xs text-amber-500/70">{role}</p>
        </div>
      </aside>
      <main className="flex-1 overflow-auto p-6">{children}</main>
    </div>
  );
}
```

- [ ] **Step 7: Create stat card component**

Create `hoags-crew-command/src/components/domain/stat-card.tsx`:

```tsx
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { LucideIcon } from "lucide-react";

interface StatCardProps {
  title: string;
  value: string | number;
  icon: LucideIcon;
  alert?: "amber" | "red";
}

export function StatCard({ title, value, icon: Icon, alert }: StatCardProps) {
  const alertClasses = {
    amber: "border-amber-500/50 bg-amber-500/5",
    red: "border-red-500/50 bg-red-500/5",
  };

  return (
    <Card className={`border-zinc-800 bg-zinc-900 ${alert ? alertClasses[alert] : ""}`}>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium text-zinc-400">{title}</CardTitle>
        <Icon className={`h-4 w-4 ${alert === "red" ? "text-red-400" : alert === "amber" ? "text-amber-400" : "text-zinc-500"}`} />
      </CardHeader>
      <CardContent>
        <div className={`text-2xl font-bold font-mono ${alert === "red" ? "text-red-400" : alert === "amber" ? "text-amber-400" : "text-zinc-100"}`}>
          {value}
        </div>
      </CardContent>
    </Card>
  );
}
```

- [ ] **Step 8: Create dashboard page**

Create `hoags-crew-command/src/app/(command)/dashboard/page.tsx`:

```tsx
import { db } from "@/lib/db";
import { StatCard } from "@/components/domain/stat-card";
import { FileText, Users, Wrench, AlertTriangle } from "lucide-react";

export default async function DashboardPage() {
  const [personnelCount, assetCount, certAlertCount] = await Promise.all([
    db.personnel.count({ where: { status: "ACTIVE" } }),
    db.asset.count({ where: { location: { not: "IN_STORAGE" } } }),
    db.certRecord.count({ where: { status: "EXPIRED" } }),
  ]);

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Dashboard</h1>
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard title="Active Contracts" value={0} icon={FileText} />
        <StatCard title="Crew Deployed" value={personnelCount} icon={Users} />
        <StatCard title="Equipment Out" value={assetCount} icon={Wrench} />
        <StatCard
          title="Cert Alerts"
          value={certAlertCount}
          icon={AlertTriangle}
          alert={certAlertCount > 0 ? (certAlertCount > 5 ? "red" : "amber") : undefined}
        />
      </div>
      <div className="rounded-lg border border-zinc-800 bg-zinc-900 p-8 text-center text-zinc-500">
        Dispatch calendar, reports, and expiring items will appear here in Phase 2.
      </div>
    </div>
  );
}
```

- [ ] **Step 9: Verify the app starts**

```bash
cd C:/Users/colli/OneDrive/Desktop/hoags-crew-command
npm run dev
```

Open `http://localhost:3000` — should redirect to `/login`. The login page should show the dark themed form.

- [ ] **Step 10: Commit**

```bash
git add src/app/ src/components/
git commit -m "feat: add command center layout, sidebar nav, dashboard shell, stat cards"
```

---

## Task 6: Personnel Server Actions

**Files:**
- Create: `hoags-crew-command/src/actions/personnel.ts`
- Create: `hoags-crew-command/tests/helpers/factories.ts`
- Create: `hoags-crew-command/tests/actions/personnel.test.ts`

- [ ] **Step 1: Create test data factories**

Create `hoags-crew-command/tests/helpers/factories.ts`:

```typescript
import type { Role, PersonnelStatus, PayType } from "@prisma/client";

let counter = 0;

export function buildPersonnel(overrides: Record<string, unknown> = {}) {
  counter++;
  return {
    firstName: `Test${counter}`,
    lastName: `User${counter}`,
    phone: `555-000-${String(counter).padStart(4, "0")}`,
    email: `test${counter}@example.com`,
    status: "APPLICANT" as PersonnelStatus,
    role: "CREW_MEMBER" as Role,
    payRate: 25.0,
    payType: "HOURLY" as PayType,
    ...overrides,
  };
}

export function buildAsset(categoryId: string, overrides: Record<string, unknown> = {}) {
  counter++;
  return {
    assetTag: `ASSET-${String(counter).padStart(4, "0")}`,
    categoryId,
    name: `Test Asset ${counter}`,
    make: "TestMake",
    model: "TestModel",
    ownership: "OWNED" as const,
    condition: "GOOD" as const,
    location: "IN_STORAGE" as const,
    ...overrides,
  };
}
```

- [ ] **Step 2: Write personnel action tests**

Create `hoags-crew-command/tests/actions/personnel.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { db } from "@/lib/db";
import { buildPersonnel } from "../helpers/factories";
import {
  createPersonnel,
  getPersonnelById,
  listPersonnel,
  updatePersonnel,
} from "@/actions/personnel";

// NOTE: These tests require a real DB connection.
// Skip if DATABASE_URL is not set.
const describeWithDb = process.env.DATABASE_URL ? describe : describe.skip;

describeWithDb("personnel actions", () => {
  beforeEach(async () => {
    await db.certRecord.deleteMany();
    await db.personnel.deleteMany();
  });

  it("creates a personnel record", async () => {
    const data = buildPersonnel();
    const result = await createPersonnel(data, "test-user");
    expect(result.id).toBeDefined();
    expect(result.firstName).toBe(data.firstName);
    expect(result.createdBy).toBe("test-user");
  });

  it("lists personnel with status filter", async () => {
    await createPersonnel(buildPersonnel({ status: "ACTIVE" }), "test-user");
    await createPersonnel(buildPersonnel({ status: "TERMINATED" }), "test-user");
    const active = await listPersonnel({ status: "ACTIVE" });
    expect(active.length).toBe(1);
    expect(active[0].status).toBe("ACTIVE");
  });

  it("gets personnel by ID with relations", async () => {
    const created = await createPersonnel(buildPersonnel(), "test-user");
    const found = await getPersonnelById(created.id);
    expect(found).not.toBeNull();
    expect(found!.id).toBe(created.id);
    expect(found!.emergencyContacts).toBeDefined();
    expect(found!.certRecords).toBeDefined();
  });

  it("updates a personnel record", async () => {
    const created = await createPersonnel(buildPersonnel(), "test-user");
    const updated = await updatePersonnel(created.id, { status: "ACTIVE" }, "test-user");
    expect(updated.status).toBe("ACTIVE");
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
npx vitest run tests/actions/personnel.test.ts
```

Expected: FAIL — `Cannot find module '@/actions/personnel'`

- [ ] **Step 4: Implement personnel actions**

Create `hoags-crew-command/src/actions/personnel.ts`:

```typescript
"use server";

import { db } from "@/lib/db";
import { auditedCreate, auditedUpdate } from "@/lib/audit";
import { encrypt } from "@/lib/encryption";
import type { PersonnelStatus, Role } from "@prisma/client";

export async function createPersonnel(
  data: {
    firstName: string;
    lastName: string;
    ssn?: string;
    dob?: string;
    address?: string;
    phone?: string;
    email?: string;
    status?: PersonnelStatus;
    role?: Role;
    payRate?: number;
    payType?: "HOURLY" | "SALARY";
    overtimeRules?: string;
    scaWageDetermId?: string;
  },
  changedBy: string,
) {
  return auditedCreate("Personnel", changedBy, () =>
    db.personnel.create({
      data: {
        firstName: data.firstName,
        lastName: data.lastName,
        ssnEncrypted: data.ssn ? encrypt(data.ssn) : null,
        dob: data.dob ? new Date(data.dob) : null,
        address: data.address,
        phone: data.phone,
        email: data.email,
        status: data.status ?? "APPLICANT",
        role: data.role ?? "CREW_MEMBER",
        payRate: data.payRate,
        payType: data.payType,
        overtimeRules: data.overtimeRules,
        scaWageDetermId: data.scaWageDetermId,
        createdBy: changedBy,
        updatedBy: changedBy,
      },
    }),
  );
}

export async function listPersonnel(filters?: {
  status?: PersonnelStatus;
  role?: Role;
  search?: string;
}) {
  return db.personnel.findMany({
    where: {
      ...(filters?.status && { status: filters.status }),
      ...(filters?.role && { role: filters.role }),
      ...(filters?.search && {
        OR: [
          { firstName: { contains: filters.search, mode: "insensitive" as const } },
          { lastName: { contains: filters.search, mode: "insensitive" as const } },
        ],
      }),
    },
    include: {
      certRecords: { include: { certType: true } },
      crewAssignments: { where: { status: "ACTIVE" }, include: { site: true } },
    },
    orderBy: { lastName: "asc" },
  });
}

export async function getPersonnelById(id: string) {
  return db.personnel.findUnique({
    where: { id },
    include: {
      emergencyContacts: true,
      documents: { orderBy: { uploadedAt: "desc" } },
      drugTests: { orderBy: { date: "desc" } },
      physicalFitness: { orderBy: { date: "desc" } },
      medicalClearances: { orderBy: { date: "desc" } },
      backgroundChecks: { orderBy: { date: "desc" } },
      availability: true,
      performanceNotes: { orderBy: { date: "desc" } },
      incidentReports: { orderBy: { date: "desc" } },
      certRecords: { include: { certType: true }, orderBy: { dateEarned: "desc" } },
      crewAssignments: { include: { site: { include: { contract: true } } }, orderBy: { startDate: "desc" } },
      chainOfCustody: { include: { asset: true }, orderBy: { checkedOutDate: "desc" } },
    },
  });
}

export async function updatePersonnel(
  id: string,
  data: Partial<{
    firstName: string;
    lastName: string;
    ssn: string;
    address: string;
    phone: string;
    email: string;
    status: PersonnelStatus;
    role: Role;
    hireDate: string;
    terminationDate: string;
    terminationReason: string;
    payRate: number;
    payType: "HOURLY" | "SALARY";
  }>,
  changedBy: string,
) {
  const old = await db.personnel.findUniqueOrThrow({ where: { id } });
  const updateData: Record<string, unknown> = { updatedBy: changedBy };
  if (data.firstName !== undefined) updateData.firstName = data.firstName;
  if (data.lastName !== undefined) updateData.lastName = data.lastName;
  if (data.ssn !== undefined) updateData.ssnEncrypted = encrypt(data.ssn);
  if (data.address !== undefined) updateData.address = data.address;
  if (data.phone !== undefined) updateData.phone = data.phone;
  if (data.email !== undefined) updateData.email = data.email;
  if (data.status !== undefined) updateData.status = data.status;
  if (data.role !== undefined) updateData.role = data.role;
  if (data.hireDate !== undefined) updateData.hireDate = new Date(data.hireDate);
  if (data.terminationDate !== undefined) updateData.terminationDate = new Date(data.terminationDate);
  if (data.terminationReason !== undefined) updateData.terminationReason = data.terminationReason;
  if (data.payRate !== undefined) updateData.payRate = data.payRate;
  if (data.payType !== undefined) updateData.payType = data.payType;

  return auditedUpdate("Personnel", changedBy, old as unknown as Record<string, unknown>, () =>
    db.personnel.update({ where: { id }, data: updateData }),
  );
}
```

- [ ] **Step 5: Run personnel tests**

```bash
npx vitest run tests/actions/personnel.test.ts
```

Expected: PASS (or SKIP if no DATABASE_URL)

- [ ] **Step 6: Commit**

```bash
git add src/actions/personnel.ts tests/
git commit -m "feat: add personnel server actions with CRUD, filtering, audit trail"
```

---

## Task 7: Personnel UI — Roster Table

**Files:**
- Create: `hoags-crew-command/src/components/domain/crew-table.tsx`
- Create: `hoags-crew-command/src/app/(command)/personnel/page.tsx`

- [ ] **Step 1: Create crew table component**

Create `hoags-crew-command/src/components/domain/crew-table.tsx`:

```tsx
"use client";

import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

interface CrewMember {
  id: string;
  firstName: string;
  lastName: string;
  role: string;
  status: string;
  phone: string | null;
  certRecords: { status: string }[];
  crewAssignments: { site: { name: string } }[];
}

const STATUS_COLORS: Record<string, string> = {
  ACTIVE: "bg-emerald-500/20 text-emerald-400 border-emerald-500/30",
  APPLICANT: "bg-blue-500/20 text-blue-400 border-blue-500/30",
  ONBOARDING: "bg-amber-500/20 text-amber-400 border-amber-500/30",
  ON_LEAVE: "bg-zinc-500/20 text-zinc-400 border-zinc-500/30",
  TERMINATED: "bg-red-500/20 text-red-400 border-red-500/30",
};

function certStatusBadge(certs: { status: string }[]) {
  if (certs.length === 0) return <Badge variant="outline" className="border-zinc-700 text-zinc-500">None</Badge>;
  const expired = certs.filter((c) => c.status === "EXPIRED").length;
  if (expired > 0) return <Badge variant="outline" className="border-red-500/30 text-red-400">{expired} Expired</Badge>;
  return <Badge variant="outline" className="border-emerald-500/30 text-emerald-400">Current</Badge>;
}

export function CrewTable({ data }: { data: CrewMember[] }) {
  return (
    <Table>
      <TableHeader>
        <TableRow className="border-zinc-800">
          <TableHead className="text-zinc-400">Name</TableHead>
          <TableHead className="text-zinc-400">Role</TableHead>
          <TableHead className="text-zinc-400">Status</TableHead>
          <TableHead className="text-zinc-400">Site</TableHead>
          <TableHead className="text-zinc-400">Certs</TableHead>
          <TableHead className="text-zinc-400">Phone</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {data.map((person) => (
          <TableRow key={person.id} className="border-zinc-800 hover:bg-zinc-800/50">
            <TableCell>
              <Link href={`/personnel/${person.id}`} className="font-medium text-zinc-100 hover:text-amber-400">
                {person.lastName}, {person.firstName}
              </Link>
            </TableCell>
            <TableCell className="font-mono text-xs text-zinc-400">{person.role.replace("_", " ")}</TableCell>
            <TableCell>
              <Badge variant="outline" className={STATUS_COLORS[person.status] ?? ""}>
                {person.status.replace("_", " ")}
              </Badge>
            </TableCell>
            <TableCell className="text-zinc-400">
              {person.crewAssignments[0]?.site.name ?? "—"}
            </TableCell>
            <TableCell>{certStatusBadge(person.certRecords)}</TableCell>
            <TableCell className="font-mono text-xs text-zinc-400">{person.phone ?? "—"}</TableCell>
          </TableRow>
        ))}
        {data.length === 0 && (
          <TableRow>
            <TableCell colSpan={6} className="py-8 text-center text-zinc-500">
              No crew members found. Add your first crew member to get started.
            </TableCell>
          </TableRow>
        )}
      </TableBody>
    </Table>
  );
}
```

- [ ] **Step 2: Create personnel roster page**

Create `hoags-crew-command/src/app/(command)/personnel/page.tsx`:

```tsx
import Link from "next/link";
import { listPersonnel } from "@/actions/personnel";
import { CrewTable } from "@/components/domain/crew-table";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { UserPlus } from "lucide-react";

export default async function PersonnelPage({
  searchParams,
}: {
  searchParams: Promise<{ status?: string; role?: string; search?: string }>;
}) {
  const params = await searchParams;
  const personnel = await listPersonnel({
    status: params.status as "ACTIVE" | undefined,
    role: params.role as "CREW_MEMBER" | undefined,
    search: params.search,
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Personnel</h1>
        <Button asChild className="bg-amber-600 text-zinc-950 hover:bg-amber-500">
          <Link href="/personnel/new">
            <UserPlus className="mr-2 h-4 w-4" />
            Add Crew Member
          </Link>
        </Button>
      </div>
      <Card className="border-zinc-800 bg-zinc-900">
        <CrewTable data={personnel as never[]} />
      </Card>
    </div>
  );
}
```

- [ ] **Step 3: Verify roster page renders**

```bash
npm run dev
```

Navigate to `http://localhost:3000/personnel` (must be logged in). Should show empty table with "No crew members found" message and an amber "Add Crew Member" button.

- [ ] **Step 4: Commit**

```bash
git add src/components/domain/crew-table.tsx src/app/\(command\)/personnel/
git commit -m "feat: add personnel roster page with filterable crew table"
```

---

## Task 8: Personnel UI — Onboarding Form + Profile

**Files:**
- Create: `hoags-crew-command/src/app/(command)/personnel/new/page.tsx`
- Create: `hoags-crew-command/src/app/(command)/personnel/[id]/page.tsx`

- [ ] **Step 1: Create onboarding form page**

Create `hoags-crew-command/src/app/(command)/personnel/new/page.tsx`:

```tsx
import { redirect } from "next/navigation";
import { auth } from "@/lib/auth";
import { createPersonnel } from "@/actions/personnel";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

export default async function NewPersonnelPage() {
  const session = await auth();

  async function handleCreate(formData: FormData) {
    "use server";
    const result = await createPersonnel(
      {
        firstName: formData.get("firstName") as string,
        lastName: formData.get("lastName") as string,
        phone: formData.get("phone") as string,
        email: formData.get("email") as string,
        role: formData.get("role") as "CREW_MEMBER",
        payRate: Number(formData.get("payRate")) || undefined,
        payType: (formData.get("payType") as "HOURLY" | "SALARY") || "HOURLY",
        address: formData.get("address") as string,
      },
      session?.user?.id ?? "system",
    );
    redirect(`/personnel/${result.id}`);
  }

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      <h1 className="text-2xl font-bold">Add Crew Member</h1>
      <Card className="border-zinc-800 bg-zinc-900">
        <CardHeader>
          <CardTitle className="text-zinc-100">Personal Information</CardTitle>
        </CardHeader>
        <CardContent>
          <form action={handleCreate} className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="firstName">First Name</Label>
                <Input id="firstName" name="firstName" required className="border-zinc-700 bg-zinc-800" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="lastName">Last Name</Label>
                <Input id="lastName" name="lastName" required className="border-zinc-700 bg-zinc-800" />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="phone">Phone</Label>
                <Input id="phone" name="phone" type="tel" className="border-zinc-700 bg-zinc-800" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="email">Email</Label>
                <Input id="email" name="email" type="email" className="border-zinc-700 bg-zinc-800" />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="address">Address</Label>
              <Input id="address" name="address" className="border-zinc-700 bg-zinc-800" />
            </div>
            <div className="grid grid-cols-3 gap-4">
              <div className="space-y-2">
                <Label htmlFor="role">Role</Label>
                <Select name="role" defaultValue="CREW_MEMBER">
                  <SelectTrigger className="border-zinc-700 bg-zinc-800">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="CREW_MEMBER">Crew Member</SelectItem>
                    <SelectItem value="CREW_LEAD">Crew Lead</SelectItem>
                    <SelectItem value="SUPERVISOR">Supervisor</SelectItem>
                    <SelectItem value="MANAGER">Manager</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="payRate">Pay Rate ($)</Label>
                <Input id="payRate" name="payRate" type="number" step="0.01" className="border-zinc-700 bg-zinc-800" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="payType">Pay Type</Label>
                <Select name="payType" defaultValue="HOURLY">
                  <SelectTrigger className="border-zinc-700 bg-zinc-800">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="HOURLY">Hourly</SelectItem>
                    <SelectItem value="SALARY">Salary</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="flex justify-end gap-3 pt-4">
              <Button variant="outline" asChild className="border-zinc-700">
                <a href="/personnel">Cancel</a>
              </Button>
              <Button type="submit" className="bg-amber-600 text-zinc-950 hover:bg-amber-500">
                Add Crew Member
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
```

- [ ] **Step 2: Create personnel profile page**

Create `hoags-crew-command/src/app/(command)/personnel/[id]/page.tsx`:

```tsx
import { notFound } from "next/navigation";
import { getPersonnelById } from "@/actions/personnel";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

export default async function PersonnelProfilePage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const person = await getPersonnelById(id);
  if (!person) notFound();

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <div className="flex h-16 w-16 items-center justify-center rounded-full bg-zinc-800 text-xl font-bold text-amber-500">
          {person.firstName[0]}{person.lastName[0]}
        </div>
        <div>
          <h1 className="text-2xl font-bold">{person.firstName} {person.lastName}</h1>
          <div className="flex items-center gap-2">
            <Badge variant="outline" className="font-mono text-xs">{person.role.replace("_", " ")}</Badge>
            <Badge variant="outline">{person.status.replace("_", " ")}</Badge>
          </div>
        </div>
      </div>

      <Tabs defaultValue="details" className="space-y-4">
        <TabsList className="border-zinc-800 bg-zinc-900">
          <TabsTrigger value="details">Details</TabsTrigger>
          <TabsTrigger value="certifications">Certifications ({person.certRecords.length})</TabsTrigger>
          <TabsTrigger value="assignments">Assignments ({person.crewAssignments.length})</TabsTrigger>
          <TabsTrigger value="documents">Documents ({person.documents.length})</TabsTrigger>
          <TabsTrigger value="drugtests">Drug Tests ({person.drugTests.length})</TabsTrigger>
          <TabsTrigger value="incidents">Incidents ({person.incidentReports.length})</TabsTrigger>
        </TabsList>

        <TabsContent value="details">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardContent className="grid gap-4 pt-6 md:grid-cols-2">
              <div>
                <p className="text-sm text-zinc-400">Phone</p>
                <p className="font-mono">{person.phone ?? "—"}</p>
              </div>
              <div>
                <p className="text-sm text-zinc-400">Email</p>
                <p className="font-mono">{person.email ?? "—"}</p>
              </div>
              <div>
                <p className="text-sm text-zinc-400">Address</p>
                <p>{person.address ?? "—"}</p>
              </div>
              <div>
                <p className="text-sm text-zinc-400">Pay</p>
                <p className="font-mono">
                  {person.payRate ? `$${Number(person.payRate).toFixed(2)}/${person.payType?.toLowerCase() ?? "hr"}` : "—"}
                </p>
              </div>
              <div>
                <p className="text-sm text-zinc-400">Hire Date</p>
                <p className="font-mono">{person.hireDate?.toLocaleDateString() ?? "—"}</p>
              </div>
              <div>
                <p className="text-sm text-zinc-400">Emergency Contacts</p>
                {person.emergencyContacts.length > 0 ? (
                  person.emergencyContacts.map((ec) => (
                    <p key={ec.id} className="text-sm">{ec.name} — {ec.phone} ({ec.relationship})</p>
                  ))
                ) : (
                  <p className="text-zinc-500">None added</p>
                )}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="certifications">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader><CardTitle>Certifications</CardTitle></CardHeader>
            <CardContent>
              {person.certRecords.length > 0 ? (
                <div className="space-y-3">
                  {person.certRecords.map((cr) => (
                    <div key={cr.id} className="flex items-center justify-between rounded-md border border-zinc-800 p-3">
                      <div>
                        <p className="font-medium">{cr.certType.name}</p>
                        <p className="text-sm text-zinc-400">
                          Earned {cr.dateEarned.toLocaleDateString()}
                          {cr.expirationDate && ` — Expires ${cr.expirationDate.toLocaleDateString()}`}
                        </p>
                      </div>
                      <Badge variant="outline" className={cr.status === "CURRENT" ? "border-emerald-500/30 text-emerald-400" : "border-red-500/30 text-red-400"}>
                        {cr.status}
                      </Badge>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="py-4 text-center text-zinc-500">No certifications recorded</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="assignments">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader><CardTitle>Site Assignments</CardTitle></CardHeader>
            <CardContent>
              {person.crewAssignments.length > 0 ? (
                <div className="space-y-3">
                  {person.crewAssignments.map((ca) => (
                    <div key={ca.id} className="flex items-center justify-between rounded-md border border-zinc-800 p-3">
                      <div>
                        <p className="font-medium">{ca.site.name}</p>
                        <p className="text-sm text-zinc-400">
                          {ca.startDate.toLocaleDateString()} — {ca.endDate?.toLocaleDateString() ?? "Ongoing"}
                        </p>
                      </div>
                      <Badge variant="outline">{ca.status}</Badge>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="py-4 text-center text-zinc-500">No assignments</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="documents">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardContent className="py-8 text-center text-zinc-500">
              Document upload will be available in Phase 2.
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="drugtests">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardContent className="py-8 text-center text-zinc-500">
              Drug test records will be available in Phase 2.
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="incidents">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardContent className="py-8 text-center text-zinc-500">
              Incident reports will be available in Phase 2.
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
```

- [ ] **Step 3: Verify the onboarding flow**

```bash
npm run dev
```

1. Go to `/personnel` → click "Add Crew Member"
2. Fill out the form → submit
3. Should redirect to the new crew member's profile page

- [ ] **Step 4: Commit**

```bash
git add src/app/\(command\)/personnel/
git commit -m "feat: add personnel onboarding form and profile page with tabs"
```

---

## Task 9: Training & Cert Types

**Files:**
- Create: `hoags-crew-command/src/actions/training.ts`
- Create: `hoags-crew-command/src/app/(command)/training/cert-types/page.tsx`
- Create: `hoags-crew-command/src/components/domain/cert-badge.tsx`
- Create: `hoags-crew-command/tests/actions/training.test.ts`

- [ ] **Step 1: Write training action tests**

Create `hoags-crew-command/tests/actions/training.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { db } from "@/lib/db";
import { createCertType, listCertTypes, addCertRecord } from "@/actions/training";
import { buildPersonnel } from "../helpers/factories";

const describeWithDb = process.env.DATABASE_URL ? describe : describe.skip;

describeWithDb("training actions", () => {
  beforeEach(async () => {
    await db.certRecord.deleteMany();
    await db.certType.deleteMany();
    await db.personnel.deleteMany();
  });

  it("creates a cert type", async () => {
    const ct = await createCertType({
      name: "S-212 Wildfire Chain Saws",
      category: "Wildfire",
      expires: true,
      validityMonths: 36,
      requiredForRoles: ["CREW_MEMBER", "CREW_LEAD"],
    });
    expect(ct.id).toBeDefined();
    expect(ct.name).toBe("S-212 Wildfire Chain Saws");
    expect(ct.requiredForRoles).toContain("CREW_MEMBER");
  });

  it("lists active cert types", async () => {
    await createCertType({ name: "Active Cert", expires: false });
    await createCertType({ name: "Inactive Cert", expires: false, isActive: false });
    const types = await listCertTypes();
    expect(types.length).toBe(1);
    expect(types[0].name).toBe("Active Cert");
  });

  it("adds a cert record to a person", async () => {
    const person = await db.personnel.create({ data: buildPersonnel() });
    const ct = await createCertType({ name: "First Aid", expires: true, validityMonths: 24 });
    const record = await addCertRecord({
      personnelId: person.id,
      certTypeId: ct.id,
      dateEarned: new Date().toISOString(),
    });
    expect(record.id).toBeDefined();
    expect(record.status).toBe("CURRENT");
    expect(record.expirationDate).not.toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run tests/actions/training.test.ts
```

Expected: FAIL — `Cannot find module '@/actions/training'`

- [ ] **Step 3: Implement training actions**

Create `hoags-crew-command/src/actions/training.ts`:

```typescript
"use server";

import { db } from "@/lib/db";
import type { Role } from "@prisma/client";

export async function createCertType(data: {
  name: string;
  category?: string;
  description?: string;
  expires: boolean;
  validityMonths?: number;
  requiredForRoles?: Role[];
  requiredForContractTypes?: string[];
  renewalRequirements?: string;
  isActive?: boolean;
}) {
  return db.certType.create({
    data: {
      name: data.name,
      category: data.category,
      description: data.description,
      expires: data.expires,
      validityMonths: data.validityMonths,
      requiredForRoles: data.requiredForRoles ?? [],
      requiredForContractTypes: data.requiredForContractTypes ?? [],
      renewalRequirements: data.renewalRequirements,
      isActive: data.isActive ?? true,
    },
  });
}

export async function listCertTypes(includeInactive = false) {
  return db.certType.findMany({
    where: includeInactive ? {} : { isActive: true },
    orderBy: { name: "asc" },
  });
}

export async function updateCertType(id: string, data: Partial<{
  name: string;
  category: string;
  description: string;
  expires: boolean;
  validityMonths: number;
  requiredForRoles: Role[];
  renewalRequirements: string;
  isActive: boolean;
}>) {
  return db.certType.update({ where: { id }, data });
}

export async function addCertRecord(data: {
  personnelId: string;
  certTypeId: string;
  dateEarned: string;
  issuingAuthority?: string;
  certNumber?: string;
  documentUrl?: string;
  notes?: string;
}) {
  const certType = await db.certType.findUniqueOrThrow({ where: { id: data.certTypeId } });
  const dateEarned = new Date(data.dateEarned);
  let expirationDate: Date | null = null;
  if (certType.expires && certType.validityMonths) {
    expirationDate = new Date(dateEarned);
    expirationDate.setMonth(expirationDate.getMonth() + certType.validityMonths);
  }
  return db.certRecord.create({
    data: {
      personnelId: data.personnelId,
      certTypeId: data.certTypeId,
      dateEarned,
      expirationDate,
      status: "CURRENT",
      issuingAuthority: data.issuingAuthority,
      certNumber: data.certNumber,
      documentUrl: data.documentUrl,
      notes: data.notes,
    },
  });
}

export async function listCertRecords(personnelId: string) {
  return db.certRecord.findMany({
    where: { personnelId },
    include: { certType: true },
    orderBy: { dateEarned: "desc" },
  });
}
```

- [ ] **Step 4: Run training tests**

```bash
npx vitest run tests/actions/training.test.ts
```

Expected: PASS (or SKIP if no DATABASE_URL)

- [ ] **Step 5: Create cert badge component**

Create `hoags-crew-command/src/components/domain/cert-badge.tsx`:

```tsx
import { Badge } from "@/components/ui/badge";

const CERT_STATUS_STYLES: Record<string, string> = {
  CURRENT: "border-emerald-500/30 text-emerald-400 bg-emerald-500/10",
  EXPIRED: "border-red-500/30 text-red-400 bg-red-500/10",
  PENDING: "border-amber-500/30 text-amber-400 bg-amber-500/10",
  REVOKED: "border-zinc-500/30 text-zinc-400 bg-zinc-500/10",
};

export function CertBadge({ status }: { status: string }) {
  return (
    <Badge variant="outline" className={CERT_STATUS_STYLES[status] ?? ""}>
      {status}
    </Badge>
  );
}
```

- [ ] **Step 6: Create cert types admin page**

Create `hoags-crew-command/src/app/(command)/training/cert-types/page.tsx`:

```tsx
import { listCertTypes, createCertType } from "@/actions/training";
import { redirect } from "next/navigation";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Plus } from "lucide-react";

export default async function CertTypesPage() {
  const certTypes = await listCertTypes(true);

  async function handleCreate(formData: FormData) {
    "use server";
    await createCertType({
      name: formData.get("name") as string,
      category: formData.get("category") as string,
      expires: formData.get("expires") === "on",
      validityMonths: Number(formData.get("validityMonths")) || undefined,
      renewalRequirements: formData.get("renewalRequirements") as string,
    });
    redirect("/training/cert-types");
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Certification Types</h1>

      <Card className="border-zinc-800 bg-zinc-900">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Plus className="h-4 w-4" /> Add Certification Type
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form action={handleCreate} className="grid gap-4 md:grid-cols-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input id="name" name="name" required placeholder="S-212 Chain Saws" className="border-zinc-700 bg-zinc-800" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="category">Category</Label>
              <Input id="category" name="category" placeholder="Wildfire" className="border-zinc-700 bg-zinc-800" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="validityMonths">Validity (months)</Label>
              <Input id="validityMonths" name="validityMonths" type="number" placeholder="36" className="border-zinc-700 bg-zinc-800" />
            </div>
            <div className="flex items-end gap-3">
              <label className="flex items-center gap-2 text-sm text-zinc-300">
                <input type="checkbox" name="expires" defaultChecked className="rounded border-zinc-700" />
                Expires
              </label>
              <Button type="submit" className="bg-amber-600 text-zinc-950 hover:bg-amber-500">Add</Button>
            </div>
          </form>
        </CardContent>
      </Card>

      <Card className="border-zinc-800 bg-zinc-900">
        <Table>
          <TableHeader>
            <TableRow className="border-zinc-800">
              <TableHead className="text-zinc-400">Name</TableHead>
              <TableHead className="text-zinc-400">Category</TableHead>
              <TableHead className="text-zinc-400">Expires</TableHead>
              <TableHead className="text-zinc-400">Validity</TableHead>
              <TableHead className="text-zinc-400">Required For</TableHead>
              <TableHead className="text-zinc-400">Status</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {certTypes.map((ct) => (
              <TableRow key={ct.id} className="border-zinc-800">
                <TableCell className="font-medium text-zinc-100">{ct.name}</TableCell>
                <TableCell className="text-zinc-400">{ct.category ?? "—"}</TableCell>
                <TableCell>{ct.expires ? "Yes" : "No"}</TableCell>
                <TableCell className="font-mono text-zinc-400">
                  {ct.validityMonths ? `${ct.validityMonths} mo` : "—"}
                </TableCell>
                <TableCell>
                  {ct.requiredForRoles.length > 0
                    ? ct.requiredForRoles.map((r) => (
                        <Badge key={r} variant="outline" className="mr-1 text-xs">{r.replace("_", " ")}</Badge>
                      ))
                    : "—"}
                </TableCell>
                <TableCell>
                  <Badge variant="outline" className={ct.isActive ? "border-emerald-500/30 text-emerald-400" : "border-zinc-600 text-zinc-500"}>
                    {ct.isActive ? "Active" : "Inactive"}
                  </Badge>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </Card>
    </div>
  );
}
```

- [ ] **Step 7: Commit**

```bash
git add src/actions/training.ts src/app/\(command\)/training/ src/components/domain/cert-badge.tsx tests/actions/training.test.ts
git commit -m "feat: add cert types admin, training actions, cert badge component"
```

---

## Task 10: Compliance Matrix

**Files:**
- Create: `hoags-crew-command/src/components/domain/compliance-matrix.tsx`
- Create: `hoags-crew-command/src/app/(command)/personnel/compliance/page.tsx`

- [ ] **Step 1: Create compliance matrix component**

Create `hoags-crew-command/src/components/domain/compliance-matrix.tsx`:

```tsx
import { Badge } from "@/components/ui/badge";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import Link from "next/link";

interface CertType {
  id: string;
  name: string;
  requiredForRoles: string[];
}

interface Person {
  id: string;
  firstName: string;
  lastName: string;
  role: string;
  certRecords: {
    certTypeId: string;
    status: string;
    expirationDate: Date | null;
  }[];
}

function cellStatus(person: Person, certType: CertType): "green" | "yellow" | "red" | "gray" {
  if (!certType.requiredForRoles.includes(person.role)) return "gray";
  const record = person.certRecords.find((cr) => cr.certTypeId === certType.id);
  if (!record) return "red";
  if (record.status === "EXPIRED") return "red";
  if (record.expirationDate) {
    const daysUntil = Math.floor((record.expirationDate.getTime() - Date.now()) / (1000 * 60 * 60 * 24));
    if (daysUntil <= 90) return "yellow";
  }
  return "green";
}

const CELL_COLORS = {
  green: "bg-emerald-500/20 border-emerald-500/30",
  yellow: "bg-amber-500/20 border-amber-500/30",
  red: "bg-red-500/20 border-red-500/30",
  gray: "bg-zinc-800/50 border-zinc-700/30",
};

export function ComplianceMatrix({ personnel, certTypes }: { personnel: Person[]; certTypes: CertType[] }) {
  return (
    <TooltipProvider>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse">
          <thead>
            <tr>
              <th className="sticky left-0 bg-zinc-900 px-3 py-2 text-left text-sm font-medium text-zinc-400">Name</th>
              {certTypes.map((ct) => (
                <th key={ct.id} className="px-2 py-2 text-center text-xs font-medium text-zinc-400" style={{ writingMode: "vertical-rl" }}>
                  {ct.name}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {personnel.map((person) => (
              <tr key={person.id} className="border-t border-zinc-800">
                <td className="sticky left-0 bg-zinc-900 px-3 py-2">
                  <Link href={`/personnel/${person.id}`} className="text-sm font-medium hover:text-amber-400">
                    {person.lastName}, {person.firstName}
                  </Link>
                  <span className="ml-2 font-mono text-xs text-zinc-500">{person.role.replace("_", " ")}</span>
                </td>
                {certTypes.map((ct) => {
                  const status = cellStatus(person, ct);
                  return (
                    <td key={ct.id} className="px-1 py-1 text-center">
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <div className={`mx-auto h-6 w-6 rounded border ${CELL_COLORS[status]}`} />
                        </TooltipTrigger>
                        <TooltipContent>
                          <p>{ct.name}: {status === "gray" ? "Not required" : status === "green" ? "Current" : status === "yellow" ? "Expiring soon" : "Missing/Expired"}</p>
                        </TooltipContent>
                      </Tooltip>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </TooltipProvider>
  );
}
```

- [ ] **Step 2: Create compliance page**

Create `hoags-crew-command/src/app/(command)/personnel/compliance/page.tsx`:

```tsx
import { db } from "@/lib/db";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ComplianceMatrix } from "@/components/domain/compliance-matrix";

export default async function CompliancePage() {
  const [personnel, certTypes] = await Promise.all([
    db.personnel.findMany({
      where: { status: { in: ["ACTIVE", "ONBOARDING"] } },
      include: { certRecords: true },
      orderBy: { lastName: "asc" },
    }),
    db.certType.findMany({
      where: { isActive: true, requiredForRoles: { isEmpty: false } },
      orderBy: { name: "asc" },
    }),
  ]);

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Compliance Matrix</h1>
      <Card className="border-zinc-800 bg-zinc-900">
        <CardHeader>
          <CardTitle className="text-sm text-zinc-400">
            Green = current | Yellow = expiring within 90 days | Red = expired/missing | Gray = not required
          </CardTitle>
        </CardHeader>
        <CardContent>
          {certTypes.length === 0 ? (
            <p className="py-8 text-center text-zinc-500">
              No cert types with role requirements defined. Go to Training → Cert Types to set up required certifications.
            </p>
          ) : (
            <ComplianceMatrix
              personnel={personnel as never[]}
              certTypes={certTypes as never[]}
            />
          )}
        </CardContent>
      </Card>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add src/components/domain/compliance-matrix.tsx src/app/\(command\)/personnel/compliance/
git commit -m "feat: add compliance matrix — crew x cert grid with color-coded status"
```

---

## Task 11: Depreciation Engine

**Files:**
- Create: `hoags-crew-command/src/lib/depreciation.ts`
- Create: `hoags-crew-command/tests/lib/depreciation.test.ts`

- [ ] **Step 1: Write depreciation tests**

Create `hoags-crew-command/tests/lib/depreciation.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { calculateBookValue, calculateMonthlyDepreciation } from "@/lib/depreciation";

describe("depreciation — straight line", () => {
  it("calculates book value at purchase", () => {
    const value = calculateBookValue({
      purchasePrice: 50000,
      purchaseDate: new Date("2026-01-01"),
      method: "STRAIGHT_LINE",
      usefulLifeMonths: 60,
      salvageValue: 5000,
      asOfDate: new Date("2026-01-01"),
    });
    expect(value).toBe(50000);
  });

  it("calculates book value after one year", () => {
    const value = calculateBookValue({
      purchasePrice: 50000,
      purchaseDate: new Date("2025-01-01"),
      method: "STRAIGHT_LINE",
      usefulLifeMonths: 60,
      salvageValue: 5000,
      asOfDate: new Date("2026-01-01"),
    });
    // Depreciable: 45000. Monthly: 750. 12 months = 9000. Book: 41000
    expect(value).toBe(41000);
  });

  it("never goes below salvage value", () => {
    const value = calculateBookValue({
      purchasePrice: 50000,
      purchaseDate: new Date("2020-01-01"),
      method: "STRAIGHT_LINE",
      usefulLifeMonths: 60,
      salvageValue: 5000,
      asOfDate: new Date("2030-01-01"),
    });
    expect(value).toBe(5000);
  });

  it("handles zero salvage value", () => {
    const value = calculateBookValue({
      purchasePrice: 12000,
      purchaseDate: new Date("2025-07-01"),
      method: "STRAIGHT_LINE",
      usefulLifeMonths: 36,
      salvageValue: 0,
      asOfDate: new Date("2026-07-01"),
    });
    // Monthly: 333.33. 12 months = 4000. Book: 8000
    expect(value).toBe(8000);
  });

  it("calculates monthly depreciation", () => {
    const monthly = calculateMonthlyDepreciation({
      purchasePrice: 50000,
      method: "STRAIGHT_LINE",
      usefulLifeMonths: 60,
      salvageValue: 5000,
    });
    expect(monthly).toBe(750);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run tests/lib/depreciation.test.ts
```

Expected: FAIL — `Cannot find module '@/lib/depreciation'`

- [ ] **Step 3: Implement depreciation engine**

Create `hoags-crew-command/src/lib/depreciation.ts`:

```typescript
export function calculateBookValue(params: {
  purchasePrice: number;
  purchaseDate: Date;
  method: "STRAIGHT_LINE" | "MACRS";
  usefulLifeMonths: number;
  salvageValue: number;
  asOfDate: Date;
}): number {
  const { purchasePrice, purchaseDate, usefulLifeMonths, salvageValue, asOfDate } = params;
  const depreciable = purchasePrice - salvageValue;
  const monthsElapsed = monthsBetween(purchaseDate, asOfDate);
  const monthlyDep = depreciable / usefulLifeMonths;
  const totalDep = Math.min(monthlyDep * monthsElapsed, depreciable);
  return Math.round(purchasePrice - totalDep);
}

export function calculateMonthlyDepreciation(params: {
  purchasePrice: number;
  method: "STRAIGHT_LINE" | "MACRS";
  usefulLifeMonths: number;
  salvageValue: number;
}): number {
  const depreciable = params.purchasePrice - params.salvageValue;
  return Math.round(depreciable / params.usefulLifeMonths);
}

function monthsBetween(start: Date, end: Date): number {
  const years = end.getFullYear() - start.getFullYear();
  const months = end.getMonth() - start.getMonth();
  return Math.max(0, years * 12 + months);
}
```

- [ ] **Step 4: Run depreciation tests**

```bash
npx vitest run tests/lib/depreciation.test.ts
```

Expected: 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/depreciation.ts tests/lib/depreciation.test.ts
git commit -m "feat: add straight-line depreciation engine with book value calculation"
```

---

## Task 12: Equipment Server Actions

**Files:**
- Create: `hoags-crew-command/src/actions/equipment.ts`
- Create: `hoags-crew-command/tests/actions/equipment.test.ts`

- [ ] **Step 1: Write equipment action tests**

Create `hoags-crew-command/tests/actions/equipment.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { db } from "@/lib/db";
import { buildAsset } from "../helpers/factories";
import {
  createAssetCategory,
  createAsset,
  listAssets,
  getAssetById,
  addFuelLog,
} from "@/actions/equipment";

const describeWithDb = process.env.DATABASE_URL ? describe : describe.skip;

describeWithDb("equipment actions", () => {
  let categoryId: string;

  beforeEach(async () => {
    await db.fuelLog.deleteMany();
    await db.workOrder.deleteMany();
    await db.inspection.deleteMany();
    await db.chainOfCustody.deleteMany();
    await db.equipmentAssignment.deleteMany();
    await db.asset.deleteMany();
    await db.assetCategory.deleteMany();
    const cat = await createAssetCategory({ name: "Vehicles", description: "Trucks and ATVs" });
    categoryId = cat.id;
  });

  it("creates an asset category", async () => {
    const cat = await createAssetCategory({
      name: "Chainsaws",
      description: "All chainsaw equipment",
      inspectionChecklistTemplate: [
        { item: "Chain tension", type: "pass_fail" },
        { item: "Bar oil level", type: "pass_fail" },
      ],
    });
    expect(cat.id).toBeDefined();
    expect(cat.name).toBe("Chainsaws");
  });

  it("creates an asset", async () => {
    const data = buildAsset(categoryId, { name: "F-250 Work Truck" });
    const asset = await createAsset(data, "test-user");
    expect(asset.id).toBeDefined();
    expect(asset.assetTag).toBe(data.assetTag);
  });

  it("lists assets with category filter", async () => {
    await createAsset(buildAsset(categoryId, { name: "Truck 1" }), "test-user");
    await createAsset(buildAsset(categoryId, { name: "Truck 2" }), "test-user");
    const assets = await listAssets({ categoryId });
    expect(assets.length).toBe(2);
  });

  it("adds a fuel log", async () => {
    const asset = await createAsset(buildAsset(categoryId), "test-user");
    const log = await addFuelLog({
      assetId: asset.id,
      date: new Date().toISOString(),
      gallons: 25.5,
      costPerGallon: 3.89,
      totalCost: 99.2,
      odometer: 45230,
    });
    expect(log.id).toBeDefined();
    expect(Number(log.gallons)).toBeCloseTo(25.5);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run tests/actions/equipment.test.ts
```

Expected: FAIL — `Cannot find module '@/actions/equipment'`

- [ ] **Step 3: Implement equipment actions**

Create `hoags-crew-command/src/actions/equipment.ts`:

```typescript
"use server";

import { db } from "@/lib/db";
import { auditedCreate, auditedUpdate } from "@/lib/audit";
import type { AssetCondition, AssetLocation, Ownership, DepreciationMethod } from "@prisma/client";

export async function createAssetCategory(data: {
  name: string;
  description?: string;
  inspectionChecklistTemplate?: unknown[];
}) {
  return db.assetCategory.create({
    data: {
      name: data.name,
      description: data.description,
      inspectionChecklistTemplate: data.inspectionChecklistTemplate ?? undefined,
    },
  });
}

export async function listAssetCategories() {
  return db.assetCategory.findMany({ orderBy: { name: "asc" } });
}

export async function createAsset(
  data: {
    assetTag: string;
    categoryId: string;
    name: string;
    make?: string;
    model?: string;
    year?: number;
    serialNumber?: string;
    vin?: string;
    purchaseDate?: string;
    purchasePrice?: number;
    vendor?: string;
    warrantyExpiry?: string;
    depreciationMethod?: DepreciationMethod;
    usefulLifeMonths?: number;
    salvageValue?: number;
    ownership?: Ownership;
    rentalSource?: string;
    rentalRate?: number;
    rentalPeriod?: string;
    condition?: AssetCondition;
    location?: AssetLocation;
  },
  changedBy: string,
) {
  return auditedCreate("Asset", changedBy, () =>
    db.asset.create({
      data: {
        assetTag: data.assetTag,
        categoryId: data.categoryId,
        name: data.name,
        make: data.make,
        model: data.model,
        year: data.year,
        serialNumber: data.serialNumber,
        vin: data.vin,
        purchaseDate: data.purchaseDate ? new Date(data.purchaseDate) : null,
        purchasePrice: data.purchasePrice,
        vendor: data.vendor,
        warrantyExpiry: data.warrantyExpiry ? new Date(data.warrantyExpiry) : null,
        depreciationMethod: data.depreciationMethod,
        usefulLifeMonths: data.usefulLifeMonths,
        salvageValue: data.salvageValue,
        ownership: data.ownership ?? "OWNED",
        rentalSource: data.rentalSource,
        rentalRate: data.rentalRate,
        rentalPeriod: data.rentalPeriod,
        condition: data.condition ?? "GOOD",
        location: data.location ?? "IN_STORAGE",
        createdBy: changedBy,
        updatedBy: changedBy,
      },
    }),
  );
}

export async function listAssets(filters?: {
  categoryId?: string;
  ownership?: Ownership;
  condition?: AssetCondition;
  location?: AssetLocation;
}) {
  return db.asset.findMany({
    where: {
      ...(filters?.categoryId && { categoryId: filters.categoryId }),
      ...(filters?.ownership && { ownership: filters.ownership }),
      ...(filters?.condition && { condition: filters.condition }),
      ...(filters?.location && { location: filters.location }),
      retiredDate: null,
    },
    include: { category: true },
    orderBy: { name: "asc" },
  });
}

export async function getAssetById(id: string) {
  return db.asset.findUnique({
    where: { id },
    include: {
      category: true,
      workOrders: { orderBy: { reportedDate: "desc" } },
      fuelLogs: { orderBy: { date: "desc" } },
      inspections: { orderBy: { date: "desc" }, include: { personnel: true } },
      chainOfCustody: { orderBy: { checkedOutDate: "desc" }, include: { personnel: true } },
    },
  });
}

export async function updateAsset(
  id: string,
  data: Partial<{
    name: string;
    condition: AssetCondition;
    location: AssetLocation;
    assignedSiteId: string | null;
    assignedPersonnelId: string | null;
  }>,
  changedBy: string,
) {
  const old = await db.asset.findUniqueOrThrow({ where: { id } });
  return auditedUpdate("Asset", changedBy, old as unknown as Record<string, unknown>, () =>
    db.asset.update({ where: { id }, data: { ...data, updatedBy: changedBy } }),
  );
}

export async function addFuelLog(data: {
  assetId: string;
  date: string;
  gallons: number;
  costPerGallon?: number;
  totalCost?: number;
  odometer?: number;
  engineHours?: number;
  location?: string;
  receiptUrl?: string;
}) {
  return db.fuelLog.create({
    data: {
      assetId: data.assetId,
      date: new Date(data.date),
      gallons: data.gallons,
      costPerGallon: data.costPerGallon,
      totalCost: data.totalCost,
      odometer: data.odometer,
      engineHours: data.engineHours,
      location: data.location,
      receiptUrl: data.receiptUrl,
    },
  });
}

export async function addWorkOrder(data: {
  assetId: string;
  type?: "SCHEDULED" | "REPORTED_ISSUE" | "RECALL";
  description: string;
  reportedBy?: string;
}) {
  return db.workOrder.create({
    data: {
      assetId: data.assetId,
      type: data.type ?? "REPORTED_ISSUE",
      description: data.description,
      reportedBy: data.reportedBy,
    },
  });
}
```

- [ ] **Step 4: Run equipment tests**

```bash
npx vitest run tests/actions/equipment.test.ts
```

Expected: PASS (or SKIP if no DATABASE_URL)

- [ ] **Step 5: Commit**

```bash
git add src/actions/equipment.ts tests/actions/equipment.test.ts
git commit -m "feat: add equipment server actions — assets, categories, fuel logs, work orders"
```

---

## Task 13: Equipment UI — Asset Table + Add Form

**Files:**
- Create: `hoags-crew-command/src/app/(command)/equipment/page.tsx`
- Create: `hoags-crew-command/src/app/(command)/equipment/new/page.tsx`
- Create: `hoags-crew-command/src/app/(command)/equipment/categories/page.tsx`

- [ ] **Step 1: Create asset table page**

Create `hoags-crew-command/src/app/(command)/equipment/page.tsx`:

```tsx
import Link from "next/link";
import { listAssets } from "@/actions/equipment";
import { calculateBookValue } from "@/lib/depreciation";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Plus } from "lucide-react";

const CONDITION_COLORS: Record<string, string> = {
  EXCELLENT: "border-emerald-500/30 text-emerald-400",
  GOOD: "border-blue-500/30 text-blue-400",
  FAIR: "border-amber-500/30 text-amber-400",
  POOR: "border-red-500/30 text-red-400",
  OUT_OF_SERVICE: "border-zinc-500/30 text-zinc-400",
};

export default async function EquipmentPage() {
  const assets = await listAssets();

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Equipment</h1>
        <div className="flex gap-2">
          <Button variant="outline" asChild className="border-zinc-700">
            <Link href="/equipment/categories">Categories</Link>
          </Button>
          <Button asChild className="bg-amber-600 text-zinc-950 hover:bg-amber-500">
            <Link href="/equipment/new"><Plus className="mr-2 h-4 w-4" />Add Asset</Link>
          </Button>
        </div>
      </div>
      <Card className="border-zinc-800 bg-zinc-900">
        <Table>
          <TableHeader>
            <TableRow className="border-zinc-800">
              <TableHead className="text-zinc-400">Asset Tag</TableHead>
              <TableHead className="text-zinc-400">Name</TableHead>
              <TableHead className="text-zinc-400">Category</TableHead>
              <TableHead className="text-zinc-400">Ownership</TableHead>
              <TableHead className="text-zinc-400">Condition</TableHead>
              <TableHead className="text-zinc-400">Location</TableHead>
              <TableHead className="text-zinc-400 text-right">Book Value</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {assets.map((asset) => {
              const bookValue =
                asset.purchasePrice && asset.purchaseDate && asset.depreciationMethod && asset.usefulLifeMonths
                  ? calculateBookValue({
                      purchasePrice: Number(asset.purchasePrice),
                      purchaseDate: asset.purchaseDate,
                      method: asset.depreciationMethod,
                      usefulLifeMonths: asset.usefulLifeMonths,
                      salvageValue: Number(asset.salvageValue ?? 0),
                      asOfDate: new Date(),
                    })
                  : null;
              return (
                <TableRow key={asset.id} className="border-zinc-800 hover:bg-zinc-800/50">
                  <TableCell className="font-mono text-xs text-amber-400">
                    <Link href={`/equipment/${asset.id}`} className="hover:underline">{asset.assetTag}</Link>
                  </TableCell>
                  <TableCell className="font-medium text-zinc-100">{asset.name}</TableCell>
                  <TableCell className="text-zinc-400">{asset.category.name}</TableCell>
                  <TableCell className="text-zinc-400">{asset.ownership}</TableCell>
                  <TableCell>
                    <Badge variant="outline" className={CONDITION_COLORS[asset.condition] ?? ""}>{asset.condition.replace("_", " ")}</Badge>
                  </TableCell>
                  <TableCell className="text-zinc-400">{asset.location.replace(/_/g, " ")}</TableCell>
                  <TableCell className="text-right font-mono text-zinc-400">
                    {bookValue !== null ? `$${bookValue.toLocaleString()}` : "—"}
                  </TableCell>
                </TableRow>
              );
            })}
            {assets.length === 0 && (
              <TableRow>
                <TableCell colSpan={7} className="py-8 text-center text-zinc-500">
                  No equipment registered. Add your first asset to get started.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </Card>
    </div>
  );
}
```

- [ ] **Step 2: Create add asset form**

Create `hoags-crew-command/src/app/(command)/equipment/new/page.tsx`:

```tsx
import { redirect } from "next/navigation";
import { auth } from "@/lib/auth";
import { createAsset, listAssetCategories } from "@/actions/equipment";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

export default async function NewAssetPage() {
  const session = await auth();
  const categories = await listAssetCategories();

  async function handleCreate(formData: FormData) {
    "use server";
    const result = await createAsset(
      {
        assetTag: formData.get("assetTag") as string,
        categoryId: formData.get("categoryId") as string,
        name: formData.get("name") as string,
        make: formData.get("make") as string,
        model: formData.get("model") as string,
        year: Number(formData.get("year")) || undefined,
        serialNumber: formData.get("serialNumber") as string,
        vin: formData.get("vin") as string,
        purchasePrice: Number(formData.get("purchasePrice")) || undefined,
        purchaseDate: (formData.get("purchaseDate") as string) || undefined,
        vendor: formData.get("vendor") as string,
        ownership: (formData.get("ownership") as "OWNED") || "OWNED",
        depreciationMethod: (formData.get("depreciationMethod") as "STRAIGHT_LINE") || undefined,
        usefulLifeMonths: Number(formData.get("usefulLifeMonths")) || undefined,
        salvageValue: Number(formData.get("salvageValue")) || undefined,
      },
      session?.user?.id ?? "system",
    );
    redirect(`/equipment/${result.id}`);
  }

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      <h1 className="text-2xl font-bold">Add Equipment</h1>
      <Card className="border-zinc-800 bg-zinc-900">
        <CardHeader><CardTitle>Asset Information</CardTitle></CardHeader>
        <CardContent>
          <form action={handleCreate} className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="assetTag">Asset Tag</Label>
                <Input id="assetTag" name="assetTag" required placeholder="VEH-001" className="border-zinc-700 bg-zinc-800 font-mono" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="categoryId">Category</Label>
                <Select name="categoryId" required>
                  <SelectTrigger className="border-zinc-700 bg-zinc-800"><SelectValue placeholder="Select category" /></SelectTrigger>
                  <SelectContent>
                    {categories.map((cat) => (
                      <SelectItem key={cat.id} value={cat.id}>{cat.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input id="name" name="name" required placeholder="2024 F-250 Work Truck" className="border-zinc-700 bg-zinc-800" />
            </div>
            <div className="grid grid-cols-3 gap-4">
              <div className="space-y-2">
                <Label htmlFor="make">Make</Label>
                <Input id="make" name="make" placeholder="Ford" className="border-zinc-700 bg-zinc-800" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="model">Model</Label>
                <Input id="model" name="model" placeholder="F-250" className="border-zinc-700 bg-zinc-800" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="year">Year</Label>
                <Input id="year" name="year" type="number" placeholder="2024" className="border-zinc-700 bg-zinc-800" />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="serialNumber">Serial Number</Label>
                <Input id="serialNumber" name="serialNumber" className="border-zinc-700 bg-zinc-800 font-mono" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="vin">VIN</Label>
                <Input id="vin" name="vin" className="border-zinc-700 bg-zinc-800 font-mono" />
              </div>
            </div>
            <div className="grid grid-cols-3 gap-4">
              <div className="space-y-2">
                <Label htmlFor="purchasePrice">Purchase Price ($)</Label>
                <Input id="purchasePrice" name="purchasePrice" type="number" step="0.01" className="border-zinc-700 bg-zinc-800" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="purchaseDate">Purchase Date</Label>
                <Input id="purchaseDate" name="purchaseDate" type="date" className="border-zinc-700 bg-zinc-800" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="ownership">Ownership</Label>
                <Select name="ownership" defaultValue="OWNED">
                  <SelectTrigger className="border-zinc-700 bg-zinc-800"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="OWNED">Owned</SelectItem>
                    <SelectItem value="RENTED">Rented</SelectItem>
                    <SelectItem value="LEASED">Leased</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="grid grid-cols-3 gap-4">
              <div className="space-y-2">
                <Label htmlFor="depreciationMethod">Depreciation</Label>
                <Select name="depreciationMethod" defaultValue="STRAIGHT_LINE">
                  <SelectTrigger className="border-zinc-700 bg-zinc-800"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="STRAIGHT_LINE">Straight Line</SelectItem>
                    <SelectItem value="MACRS">MACRS</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="usefulLifeMonths">Useful Life (months)</Label>
                <Input id="usefulLifeMonths" name="usefulLifeMonths" type="number" placeholder="60" className="border-zinc-700 bg-zinc-800" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="salvageValue">Salvage Value ($)</Label>
                <Input id="salvageValue" name="salvageValue" type="number" step="0.01" placeholder="5000" className="border-zinc-700 bg-zinc-800" />
              </div>
            </div>
            <div className="flex justify-end gap-3 pt-4">
              <Button variant="outline" asChild className="border-zinc-700"><a href="/equipment">Cancel</a></Button>
              <Button type="submit" className="bg-amber-600 text-zinc-950 hover:bg-amber-500">Add Asset</Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
```

- [ ] **Step 3: Create asset categories page**

Create `hoags-crew-command/src/app/(command)/equipment/categories/page.tsx`:

```tsx
import { listAssetCategories, createAssetCategory } from "@/actions/equipment";
import { redirect } from "next/navigation";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Plus } from "lucide-react";

export default async function AssetCategoriesPage() {
  const categories = await listAssetCategories();

  async function handleCreate(formData: FormData) {
    "use server";
    await createAssetCategory({
      name: formData.get("name") as string,
      description: formData.get("description") as string,
    });
    redirect("/equipment/categories");
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Asset Categories</h1>
      <Card className="border-zinc-800 bg-zinc-900">
        <CardHeader><CardTitle className="flex items-center gap-2"><Plus className="h-4 w-4" /> Add Category</CardTitle></CardHeader>
        <CardContent>
          <form action={handleCreate} className="flex gap-4">
            <div className="flex-1 space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input id="name" name="name" required placeholder="Vehicles" className="border-zinc-700 bg-zinc-800" />
            </div>
            <div className="flex-1 space-y-2">
              <Label htmlFor="description">Description</Label>
              <Input id="description" name="description" placeholder="Trucks, ATVs, trailers" className="border-zinc-700 bg-zinc-800" />
            </div>
            <div className="flex items-end">
              <Button type="submit" className="bg-amber-600 text-zinc-950 hover:bg-amber-500">Add</Button>
            </div>
          </form>
        </CardContent>
      </Card>
      <Card className="border-zinc-800 bg-zinc-900">
        <Table>
          <TableHeader>
            <TableRow className="border-zinc-800">
              <TableHead className="text-zinc-400">Name</TableHead>
              <TableHead className="text-zinc-400">Description</TableHead>
              <TableHead className="text-zinc-400">Assets</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {categories.map((cat) => (
              <TableRow key={cat.id} className="border-zinc-800">
                <TableCell className="font-medium text-zinc-100">{cat.name}</TableCell>
                <TableCell className="text-zinc-400">{cat.description ?? "—"}</TableCell>
                <TableCell className="font-mono text-zinc-400">{cat.assets?.length ?? 0}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </Card>
    </div>
  );
}
```

- [ ] **Step 4: Commit**

```bash
git add src/app/\(command\)/equipment/
git commit -m "feat: add equipment pages — asset table, add form, category admin"
```

---

## Task 14: Equipment Detail Page

**Files:**
- Create: `hoags-crew-command/src/app/(command)/equipment/[id]/page.tsx`

- [ ] **Step 1: Create asset detail page with tabs**

Create `hoags-crew-command/src/app/(command)/equipment/[id]/page.tsx`:

```tsx
import { notFound } from "next/navigation";
import { getAssetById } from "@/actions/equipment";
import { calculateBookValue, calculateMonthlyDepreciation } from "@/lib/depreciation";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

export default async function AssetDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const asset = await getAssetById(id);
  if (!asset) notFound();

  const bookValue =
    asset.purchasePrice && asset.purchaseDate && asset.depreciationMethod && asset.usefulLifeMonths
      ? calculateBookValue({
          purchasePrice: Number(asset.purchasePrice),
          purchaseDate: asset.purchaseDate,
          method: asset.depreciationMethod,
          usefulLifeMonths: asset.usefulLifeMonths,
          salvageValue: Number(asset.salvageValue ?? 0),
          asOfDate: new Date(),
        })
      : null;

  const monthlyDep =
    asset.purchasePrice && asset.depreciationMethod && asset.usefulLifeMonths
      ? calculateMonthlyDepreciation({
          purchasePrice: Number(asset.purchasePrice),
          method: asset.depreciationMethod,
          usefulLifeMonths: asset.usefulLifeMonths,
          salvageValue: Number(asset.salvageValue ?? 0),
        })
      : null;

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <div className="flex h-16 w-16 items-center justify-center rounded-lg bg-zinc-800 font-mono text-sm font-bold text-amber-500">
          {asset.assetTag}
        </div>
        <div>
          <h1 className="text-2xl font-bold">{asset.name}</h1>
          <div className="flex items-center gap-2 text-sm text-zinc-400">
            <span>{asset.category.name}</span>
            <span>|</span>
            <Badge variant="outline">{asset.condition.replace("_", " ")}</Badge>
            <span>|</span>
            <span>{asset.location.replace(/_/g, " ")}</span>
          </div>
        </div>
        {bookValue !== null && (
          <div className="ml-auto text-right">
            <p className="text-sm text-zinc-400">Book Value</p>
            <p className="font-mono text-xl font-bold text-zinc-100">${bookValue.toLocaleString()}</p>
            {monthlyDep !== null && <p className="font-mono text-xs text-zinc-500">-${monthlyDep}/mo depreciation</p>}
          </div>
        )}
      </div>

      <Tabs defaultValue="info" className="space-y-4">
        <TabsList className="border-zinc-800 bg-zinc-900">
          <TabsTrigger value="info">Info</TabsTrigger>
          <TabsTrigger value="maintenance">Maintenance ({asset.workOrders.length})</TabsTrigger>
          <TabsTrigger value="fuel">Fuel Log ({asset.fuelLogs.length})</TabsTrigger>
          <TabsTrigger value="inspections">Inspections ({asset.inspections.length})</TabsTrigger>
          <TabsTrigger value="custody">Chain of Custody ({asset.chainOfCustody.length})</TabsTrigger>
        </TabsList>

        <TabsContent value="info">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardContent className="grid gap-4 pt-6 md:grid-cols-3">
              <div><p className="text-sm text-zinc-400">Make</p><p>{asset.make ?? "—"}</p></div>
              <div><p className="text-sm text-zinc-400">Model</p><p>{asset.model ?? "—"}</p></div>
              <div><p className="text-sm text-zinc-400">Year</p><p className="font-mono">{asset.year ?? "—"}</p></div>
              <div><p className="text-sm text-zinc-400">Serial Number</p><p className="font-mono">{asset.serialNumber ?? "—"}</p></div>
              <div><p className="text-sm text-zinc-400">VIN</p><p className="font-mono">{asset.vin ?? "—"}</p></div>
              <div><p className="text-sm text-zinc-400">Ownership</p><p>{asset.ownership}</p></div>
              <div><p className="text-sm text-zinc-400">Purchase Price</p><p className="font-mono">{asset.purchasePrice ? `$${Number(asset.purchasePrice).toLocaleString()}` : "—"}</p></div>
              <div><p className="text-sm text-zinc-400">Purchase Date</p><p className="font-mono">{asset.purchaseDate?.toLocaleDateString() ?? "—"}</p></div>
              <div><p className="text-sm text-zinc-400">Vendor</p><p>{asset.vendor ?? "—"}</p></div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="maintenance">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader><CardTitle>Work Orders</CardTitle></CardHeader>
            <CardContent>
              {asset.workOrders.length > 0 ? (
                <Table>
                  <TableHeader>
                    <TableRow className="border-zinc-800">
                      <TableHead className="text-zinc-400">Date</TableHead>
                      <TableHead className="text-zinc-400">Type</TableHead>
                      <TableHead className="text-zinc-400">Description</TableHead>
                      <TableHead className="text-zinc-400">Status</TableHead>
                      <TableHead className="text-zinc-400 text-right">Cost</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {asset.workOrders.map((wo) => (
                      <TableRow key={wo.id} className="border-zinc-800">
                        <TableCell className="font-mono text-xs">{wo.reportedDate.toLocaleDateString()}</TableCell>
                        <TableCell className="text-zinc-400">{wo.type.replace("_", " ")}</TableCell>
                        <TableCell>{wo.description}</TableCell>
                        <TableCell><Badge variant="outline">{wo.status.replace("_", " ")}</Badge></TableCell>
                        <TableCell className="text-right font-mono">{wo.totalCost ? `$${Number(wo.totalCost).toFixed(2)}` : "—"}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              ) : (
                <p className="py-4 text-center text-zinc-500">No work orders</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="fuel">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader><CardTitle>Fuel Log</CardTitle></CardHeader>
            <CardContent>
              {asset.fuelLogs.length > 0 ? (
                <Table>
                  <TableHeader>
                    <TableRow className="border-zinc-800">
                      <TableHead className="text-zinc-400">Date</TableHead>
                      <TableHead className="text-zinc-400">Gallons</TableHead>
                      <TableHead className="text-zinc-400">$/gal</TableHead>
                      <TableHead className="text-zinc-400">Total</TableHead>
                      <TableHead className="text-zinc-400">Odometer</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {asset.fuelLogs.map((fl) => (
                      <TableRow key={fl.id} className="border-zinc-800">
                        <TableCell className="font-mono text-xs">{fl.date.toLocaleDateString()}</TableCell>
                        <TableCell className="font-mono">{Number(fl.gallons).toFixed(1)}</TableCell>
                        <TableCell className="font-mono">{fl.costPerGallon ? `$${Number(fl.costPerGallon).toFixed(2)}` : "—"}</TableCell>
                        <TableCell className="font-mono">{fl.totalCost ? `$${Number(fl.totalCost).toFixed(2)}` : "—"}</TableCell>
                        <TableCell className="font-mono">{fl.odometer?.toLocaleString() ?? "—"}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              ) : (
                <p className="py-4 text-center text-zinc-500">No fuel logs</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="inspections">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardContent className="py-8 text-center text-zinc-500">
              Inspections will be available in the field view (Phase 2).
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="custody">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader><CardTitle>Chain of Custody</CardTitle></CardHeader>
            <CardContent>
              {asset.chainOfCustody.length > 0 ? (
                <div className="space-y-3">
                  {asset.chainOfCustody.map((coc) => (
                    <div key={coc.id} className="flex items-center justify-between rounded-md border border-zinc-800 p-3">
                      <div>
                        <p className="font-medium">{coc.personnel.firstName} {coc.personnel.lastName}</p>
                        <p className="text-sm text-zinc-400">
                          Out: {coc.checkedOutDate.toLocaleDateString()} — In: {coc.checkedInDate?.toLocaleDateString() ?? "Still out"}
                        </p>
                      </div>
                      <Badge variant="outline">{coc.checkedInDate ? "Returned" : "Checked Out"}</Badge>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="py-4 text-center text-zinc-500">No custody records</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
```

- [ ] **Step 2: Verify asset detail page**

```bash
npm run dev
```

Create an asset category, then add an asset. Navigate to its detail page. Should show asset info, tabs for maintenance/fuel/inspections/custody.

- [ ] **Step 3: Commit**

```bash
git add src/app/\(command\)/equipment/\[id\]/
git commit -m "feat: add asset detail page with info, maintenance, fuel, inspection, custody tabs"
```

---

## Task 15: Seed Script + Smoke Test

**Files:**
- Create: `hoags-crew-command/prisma/seed.ts`

- [ ] **Step 1: Install tsx for running seed**

```bash
npm install -D tsx
```

- [ ] **Step 2: Create seed script**

Create `hoags-crew-command/prisma/seed.ts`:

```typescript
import { PrismaClient } from "@prisma/client";
import { hash } from "bcryptjs";

const db = new PrismaClient();

async function main() {
  console.log("Seeding database...");

  // Owner user
  const hashedPassword = await hash("crewcommand2026", 12);
  const owner = await db.user.upsert({
    where: { email: "collin@hoagsinc.com" },
    update: {},
    create: { email: "collin@hoagsinc.com", hashedPassword, role: "OWNER" },
  });
  console.log("  Created owner:", owner.email);

  // Asset categories
  const vehicles = await db.assetCategory.create({ data: { name: "Vehicles", description: "Trucks, ATVs, trailers" } });
  const saws = await db.assetCategory.create({ data: { name: "Chainsaws", description: "All chainsaw equipment" } });
  const ppe = await db.assetCategory.create({ data: { name: "PPE", description: "Personal protective equipment" } });
  const comms = await db.assetCategory.create({ data: { name: "Communications", description: "Radios, phones, GPS" } });
  console.log("  Created 4 asset categories");

  // Cert types
  const s212 = await db.certType.create({
    data: { name: "S-212 Wildfire Chain Saws", category: "Wildfire", expires: true, validityMonths: 36, requiredForRoles: ["CREW_MEMBER", "CREW_LEAD"] },
  });
  const s130 = await db.certType.create({
    data: { name: "S-130/S-190 Basic Firefighter", category: "Wildfire", expires: true, validityMonths: 12, requiredForRoles: ["CREW_MEMBER", "CREW_LEAD", "SUPERVISOR"] },
  });
  const firstAid = await db.certType.create({
    data: { name: "First Aid/CPR", category: "Medical", expires: true, validityMonths: 24, requiredForRoles: ["CREW_MEMBER", "CREW_LEAD", "SUPERVISOR"] },
  });
  const drugTest = await db.certType.create({
    data: { name: "Drug Test (Pre-Employment)", category: "Compliance", expires: false, requiredForRoles: ["CREW_MEMBER", "CREW_LEAD", "SUPERVISOR"] },
  });
  console.log("  Created 4 cert types");

  // Sample crew
  const crew1 = await db.personnel.create({
    data: {
      firstName: "Jake", lastName: "Torres", phone: "505-555-0101", email: "jake@example.com",
      status: "ACTIVE", role: "CREW_LEAD", payRate: 28.50, payType: "HOURLY", hireDate: new Date("2025-06-01"),
      createdBy: owner.id, updatedBy: owner.id,
    },
  });
  const crew2 = await db.personnel.create({
    data: {
      firstName: "Maria", lastName: "Sandoval", phone: "505-555-0102", email: "maria@example.com",
      status: "ACTIVE", role: "CREW_MEMBER", payRate: 24.00, payType: "HOURLY", hireDate: new Date("2025-07-15"),
      createdBy: owner.id, updatedBy: owner.id,
    },
  });
  const crew3 = await db.personnel.create({
    data: {
      firstName: "Ben", lastName: "Hawkins", phone: "505-555-0103",
      status: "ONBOARDING", role: "CREW_MEMBER", payRate: 22.00, payType: "HOURLY",
      createdBy: owner.id, updatedBy: owner.id,
    },
  });
  console.log("  Created 3 crew members");

  // Cert records
  await db.certRecord.createMany({
    data: [
      { personnelId: crew1.id, certTypeId: s212.id, dateEarned: new Date("2025-03-01"), expirationDate: new Date("2028-03-01"), status: "CURRENT" },
      { personnelId: crew1.id, certTypeId: firstAid.id, dateEarned: new Date("2025-01-15"), expirationDate: new Date("2027-01-15"), status: "CURRENT" },
      { personnelId: crew2.id, certTypeId: firstAid.id, dateEarned: new Date("2024-06-01"), expirationDate: new Date("2026-06-01"), status: "CURRENT" },
      // Maria is missing S-212 — will show red on compliance matrix
    ],
  });
  console.log("  Created cert records");

  // Sample assets
  await db.asset.createMany({
    data: [
      {
        assetTag: "VEH-001", categoryId: vehicles.id, name: "2024 F-250 Work Truck", make: "Ford", model: "F-250", year: 2024,
        purchasePrice: 52000, purchaseDate: new Date("2024-01-15"), ownership: "OWNED", condition: "EXCELLENT", location: "IN_STORAGE",
        depreciationMethod: "STRAIGHT_LINE", usefulLifeMonths: 84, salvageValue: 12000, createdBy: owner.id, updatedBy: owner.id,
      },
      {
        assetTag: "VEH-002", categoryId: vehicles.id, name: "Polaris Ranger ATV", make: "Polaris", model: "Ranger 570", year: 2023,
        purchasePrice: 14500, purchaseDate: new Date("2023-06-01"), ownership: "OWNED", condition: "GOOD", location: "IN_STORAGE",
        depreciationMethod: "STRAIGHT_LINE", usefulLifeMonths: 60, salvageValue: 3000, createdBy: owner.id, updatedBy: owner.id,
      },
      {
        assetTag: "SAW-001", categoryId: saws.id, name: "Stihl MS 462", make: "Stihl", model: "MS 462", year: 2025,
        purchasePrice: 1100, purchaseDate: new Date("2025-02-01"), ownership: "OWNED", condition: "EXCELLENT", location: "IN_STORAGE",
        depreciationMethod: "STRAIGHT_LINE", usefulLifeMonths: 36, salvageValue: 200, createdBy: owner.id, updatedBy: owner.id,
      },
      {
        assetTag: "VEH-003", categoryId: vehicles.id, name: "Rental F-150 (Sandia)", make: "Ford", model: "F-150", year: 2025,
        ownership: "RENTED", rentalSource: "Enterprise Fleet", rentalRate: 85, rentalPeriod: "Daily",
        condition: "GOOD", location: "IN_STORAGE", createdBy: owner.id, updatedBy: owner.id,
      },
    ],
  });
  console.log("  Created 4 assets");

  console.log("Seed complete!");
}

main()
  .catch((e) => { console.error(e); process.exit(1); })
  .finally(() => db.$disconnect());
```

- [ ] **Step 3: Run seed**

```bash
cd C:/Users/colli/OneDrive/Desktop/hoags-crew-command
npx tsx prisma/seed.ts
```

Expected: "Seed complete!" with counts for each entity.

- [ ] **Step 4: Run all unit tests**

```bash
npx vitest run
```

Expected: All unit tests PASS (encryption, permissions, depreciation). Integration tests may skip without DB.

- [ ] **Step 5: Manual smoke test**

```bash
npm run dev
```

1. Login with `collin@hoagsinc.com` / `crewcommand2026`
2. Dashboard shows stat cards
3. `/personnel` — shows 3 crew members
4. Click Jake Torres — profile with cert records
5. `/personnel/compliance` — matrix shows Jake green, Maria yellow/red, Ben red
6. `/training/cert-types` — shows 4 cert types
7. `/equipment` — shows 4 assets with book values
8. Click VEH-001 — detail page with depreciation info
9. `/equipment/categories` — shows 4 categories

- [ ] **Step 6: Commit**

```bash
git add prisma/seed.ts
git commit -m "feat: add seed script with demo crew, certs, equipment for dev/demo"
```

---

## Phase 1 Summary

After completing all 15 tasks, you have:

- **Working app** at `http://localhost:3000` with auth, dark theme, sidebar nav
- **Personnel module** — roster, onboarding, profiles with tabs, compliance matrix
- **Training module** — admin-defined cert types, cert records, expiration tracking
- **Equipment module** — asset registry with categories, depreciation, fuel logs, work orders, chain of custody
- **Core infrastructure** — Prisma schema (full — all tables for Phase 2+), encrypted PII, role-based permissions, audit trail on all writes
- **Tests** — unit tests for encryption, permissions, depreciation; integration tests for server actions
- **Seed data** — demo crew, certs, and equipment for development

## Phase 2 Preview (Separate Plan)

- Insurance & COI module
- Contracts & Sites module
- Dispatch board (calendar + drag-and-drop)
- Daily reports (supervisor field view)
- Notifications/alerts engine (cron)
- Documents & RAG (upload, embed, query)
- Reports (financial, compliance, utilization)
- E2E tests with Playwright
