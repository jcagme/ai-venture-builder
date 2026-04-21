# ContractIQ MVP Design Document

## Overview

ContractIQ is an AI-powered contract management platform enabling SMBs to automatically extract, analyze, and track vendor/client contracts without legal expertise. The MVP focuses on core value: intelligent document processing, risk flagging, and renewal tracking.

**Target Launch**: 12-16 weeks  
**Initial Users**: 100-500 SMB operators  
**Success Metrics**: 
- Contract upload success rate >95%
- Key term extraction accuracy >90%
- User retention >60% after 30 days

---

## Tech Stack

### Backend
- **Language**: Python 3.11
- **Framework**: FastAPI (lightweight, async-native, auto-docs)
- **Task Queue**: Celery + Redis (async document processing)
- **AI/NLP**: OpenAI API (GPT-4 Turbo) for extraction and risk analysis
- **Document Processing**: PyPDF2, python-docx, google-python-client
- **Auth**: JWT tokens via python-jose
- **Testing**: pytest, pytest-asyncio

### Frontend
- **Framework**: React 18 with TypeScript
- **UI Components**: shadcn/ui (Tailwind CSS)
- **State Management**: TanStack Query + Zustand
- **Date Handling**: date-fns
- **File Upload**: react-dropzone

### DevOps
- **Containerization**: Docker + Docker Compose
- **Package Manager**: Poetry (Python), pnpm (Node)
- **Linting**: Ruff, ESLint
- **Type Checking**: mypy, TypeScript

---

## Database Design

### SQL vs NoSQL Decision
**Decision**: PostgreSQL (SQL)

**Rationale**:
- Strong ACID guarantees for financial/legal data
- Complex relational queries (contracts ↔ parties ↔ renewal dates)
- Structured data with predictable schema
- Native JSON support for flexible contract metadata
- Better for compliance auditing

### Schema

```sql
-- Users & Teams
CREATE TABLE users (
  id UUID PRIMARY KEY,
  email VARCHAR(255) UNIQUE NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE teams (
  id UUID PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  owner_id UUID NOT NULL REFERENCES users(id),
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE team_members (
  id UUID PRIMARY KEY,
  team_id UUID NOT NULL REFERENCES teams(id),
  user_id UUID NOT NULL REFERENCES users(id),
  role VARCHAR(50) DEFAULT 'member',
  UNIQUE(team_id, user_id)
);

-- Contracts
CREATE TABLE contracts (
  id UUID PRIMARY KEY,
  team_id UUID NOT NULL REFERENCES teams(id),
  file_name VARCHAR(255) NOT NULL,
  file_url VARCHAR(512) NOT NULL,
  file_type VARCHAR(20),
  status VARCHAR(50) DEFAULT 'processing', -- processing, ready, failed
  created_at TIMESTAMP DEFAULT NOW(),
  updated_at TIMESTAMP DEFAULT NOW()
);

-- Extracted Contract Data
CREATE TABLE contract_metadata (
  id UUID PRIMARY KEY,
  contract_id UUID UNIQUE NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
  parties JSONB, -- [{name, type (vendor/client)}]
  contract_value DECIMAL(15,2),
  currency VARCHAR(3),
  start_date DATE,
  end_date DATE,
  renewal_date DATE,
  renewal_type VARCHAR(50), -- auto_renewal, manual
  payment_terms JSONB, -- {frequency, amount, method}
  extracted_at TIMESTAMP DEFAULT NOW()
);

-- Risk Flags
CREATE TABLE risk_flags (
  id UUID PRIMARY KEY,
  contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
  risk_type VARCHAR(100), -- auto_renewal, unfavorable_payment, etc
  severity VARCHAR(20), -- high, medium, low
  description TEXT,
  suggested_action TEXT,
  flagged_at TIMESTAMP DEFAULT NOW()
);

-- Renewal Tracking
CREATE TABLE renewal_reminders (
  id UUID PRIMARY KEY,
  contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
  team_id UUID NOT NULL REFERENCES teams(id),
  renewal_date DATE NOT NULL,
  reminder_sent BOOLEAN DEFAULT FALSE,
  sent_at TIMESTAMP,
  created_at TIMESTAMP DEFAULT NOW()
);

-- Audit Log
CREATE TABLE audit_logs (
  id UUID PRIMARY KEY,
  team_id UUID NOT NULL REFERENCES teams(id),
  action VARCHAR(100),
  resource_id UUID,
  user_id UUID REFERENCES users(id),
  timestamp TIMESTAMP DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_contracts_team ON contracts(team_id);
CREATE INDEX idx_contract_metadata_contract ON contract_metadata(contract_id);
CREATE INDEX idx_risk_flags_contract ON risk_flags(contract_id);
CREATE INDEX idx_renewal_reminders_date ON renewal_reminders(renewal_date);
CREATE INDEX idx_renewal_reminders_team ON renewal_reminders(team_id);
```

---

## Architecture

### High-Level Components

```
┌─────────────────────────────────────────────────────────────┐
│                     React Frontend (SPA)                     │
│         (Login, Dashboard, Contract Upload, Calendar)       │
└────────────────────┬────────────────────────────────────────┘
                     │ REST API
┌────────────────────▼────────────────────────────────────────┐
│                   FastAPI Backend                            │
│  ┌──────────────┐  ┌─────────────┐  ┌──────────────────┐   │
│  │Auth Service  │  │Contract API │  │Dashboard API     │   │
│  └──────────────┘  └──────┬──────┘  └──────────────────┘   │
└─────────────────────────┬──┴──────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
┌───────▼──────┐  ┌──────▼──────┐  ┌───────▼──────┐
│ File Upload  │  │  Document   │  │ PostgreSQL   │
│ Service      │  │  Processor  │  │ Database     │
│ (S3/GCS)     │  │  (Celery)   │  │              │
└──────────────┘  └──────┬──────┘  └──────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
   ┌────▼────┐    ┌──────▼───┐    ┌──────▼────┐
   │OpenAI   │    │Email     │    │Redis      │
   │API      │    │Service   │    │Cache      │
   └─────────┘    └──────────┘    └───────────┘
```

### Key Services

**1. Auth Service**: JWT-based user/team authentication, role-based access control

**2. Contract Service**: CRUD operations, soft deletes, audit logging

**3. Document Processor (Async)**:
- Convert PDF/DOCX to text
- Chunk large documents (OpenAI token limits)
- Call OpenAI API with structured prompts
- Store extracted metadata
- Trigger risk analysis

**4. Risk Analyzer**: Rule-based + AI flagging
- Auto-renewal detection
- Unfavorable terms (e.g., 90+ day payment terms)
- Missing key terms

**5. Notification Service**: Email reminders 7/14/30 days before renewal

---

## Deployment Plan

### Containerization: Docker

**Dockerfile (Backend)**
```dockerfile
FROM python:3.11-slim
WORKDIR /app
COPY pyproject.toml poetry.lock ./
RUN pip install poetry && poetry install --no-root
COPY . .
CMD ["uvicorn", "main:app", "--host", ""0.0.0.0", "--port", "8000"]
```

**Docker Compose** (Local + Staging)
```yaml
version: '3.9'
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: contractiq
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

  backend:
    build: ./backend
    ports:
      - "8000:8000"
    environment:
      DATABASE_URL: postgresql://postgres:${DB_PASSWORD}@postgres:5432/contractiq
      REDIS_URL: redis://redis:6379
      OPENAI_API_KEY: ${OPENAI_API_KEY}
    depends_on:
      - postgres
      - redis

  celery:
    build: ./backend
    command: celery -A tasks worker --loglevel=info
    environment:
      DATABASE_URL: postgresql://postgres:${DB_PASSWORD}@postgres:5432/contractiq
      REDIS_URL: redis://redis:6379
      OPENAI_API_KEY: ${OPENAI_API_KEY}
    depends_on:
      - postgres
      - redis

  frontend:
    build: ./frontend
    ports:
      - "3000:3000"
    environment:
      REACT_APP_API_URL: http://localhost:8000
```

### CI/CD Pipeline

**Tool**: GitHub Actions

```yaml
# .github/workflows/deploy.yml
name: Deploy ContractIQ

on:
  push:
    branches: [main, staging]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: testpass
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3
      
      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'
          cache: 'poetry'

      - name: Install dependencies
        run: |
          pip install poetry
          poetry install

      - name: Lint
        run: poetry run ruff check .

      - name: Type check
        run: poetry run mypy backend/

      - name: Run tests
        run: poetry run pytest -v --cov

      - name: Set up Node
        uses: actions/setup-node@v3
        with:
          node-version: '18'
          cache: 'pnpm'

      - name: Frontend lint
        run: cd frontend && pnpm lint

      - name: Frontend build
        run: cd frontend && pnpm build

  build-and-push:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'

    steps:
      - uses: actions/checkout@v3

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v2

      - name: Login to Docker Hub
        uses: docker/login-action@v2
        with:
          username: ${{ secrets.DOCKER_USERNAME }}
          password: ${{ secrets.DOCKER_PASSWORD }}

      - name: Build and push backend
        uses: docker/build-push-action@v4
        with:
          context: ./backend
          push: true
          tags: contractiq/backend:${{ github.sha }},contractiq/backend:latest

      - name: Build and push frontend
        uses: docker/build-push-action@v4
        with:
          context: ./frontend
          push: true
          tags: contractiq/frontend:${{ github.sha }},contractiq/frontend:latest

  deploy:
    needs: build-and-push
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'

    steps:
      - uses: actions/checkout@v3

      - name: Deploy to production
        run: |
          curl -X POST ${{ secrets.DEPLOY_WEBHOOK }} \
            -H "Authorization: Bearer ${{ secrets.DEPLOY_TOKEN }}" \
            -d '{"image_tag":"${{ github.sha }}"}'
```

---

## Infrastructure

### Cloud Provider: Google Cloud Platform (GCP)

**Rationale**: Strong free tier, good PDF handling, accessible for SMB-scale spending

### Services

```
┌─────────────────────────────────────────────────────┐
│            Google Cloud Run (Backend)                │
│      • 2 CPU, 4GB RAM per instance                  │
│      • Auto-scaling (0-100 instances)               │
│      • ~$0.00002/request + compute time             │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│      Cloud Storage (File uploads)                    │
│      • Contract PDFs & DOCX files                   │
│      • Signed URLs for secure access                │
│      • ~$0.020/GB/month                             │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│      Cloud SQL (PostgreSQL)                         │
│      • db-g1-small (~$6/day)                        │
│      • Automated backups, SSL                       │
│      • 10GB SSD storage                             │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│      Cloud Memorystore (Redis)                      │
│      • 1GB basic tier (~$2/day)                     │
│      • For Celery queue & session caching           │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│      Cloud Tasks (Scheduled jobs)                    │
│      • Daily renewal reminder checks                │
│      • Cleanup expired tokens                       │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│      Firebase Hosting (Frontend)                     │
│      • CDN, auto-HTTPS, free tier                   │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│      Secret Manager                                  │
│      • Store API keys, DB passwords                 │
└─────────────────────────────────────────────────────┘
```

### Estimated MVP Monthly Cost
- Cloud Run: $15-40
- Cloud SQL: $180-200
- Redis: $60
- Storage: $10-20
- CDN/Hosting: Free
- **Total: ~$265-320/month** (scales with usage)

---

## Task Breakdown

### Engineering Task Roadmap (MVP: 12-16 weeks)

#### Phase 1: Foundation (Weeks 1-3)

**Task 1.1: Project Setup & Infrastructure**
- Initialize FastAPI + React projects with Poetry/pnpm
- Configure Docker, Docker Compose, GitHub Actions
- Set up GCP project, Cloud SQL, Cloud Storage buckets
- Create base database schema and migrations (Alembic)
- **Deliverable**: Local dev environment works end-to-end
- **Owner**: 1 DevOps/Backend engineer
- **Effort**: 40 hours

**Task 1.2: Authentication & User Management**
- Implement JWT-based auth (register, login, logout)
- User/Team creation and management APIs
- Role-based access control (RBAC)
- Protected routes in FastAPI and React
- **Deliverable**: Secure auth flow, no unauthed requests possible
- **Owner**: 1 Backend engineer
- **Effort**: 35 hours

#### Phase 2: Core Contract Management (Weeks 4-7)

**Task 2.1: File Upload & Storage**
- Implement file upload endpoint (PDF, DOCX, Google Docs)
- Validate file types and size limits (50MB max)
- Upload to Cloud Storage with signed URLs
- Create contract records in database
- Build React upload component with drag-and-drop
- **Deliverable**: Users can upload 5 contract types, view file list
- **Owner**: 1 Backend + 1 Frontend engineer
- **Effort**: 50 hours

**Task 2.2: Document Processing Pipeline**
- Build document-to-text converter (PyPDF2, python-docx)
- Implement text chunking for large documents (OpenAI token limits)
- Create Celery task for async processing
- Add error handling and retry logic
- Build processing status UI (in-progress, completed, failed)
- **Deliverable**: Uploaded contracts convert to text without errors
- **Owner**: 1 Backend engineer
- **Effort**: 45 hours

**Task 2.3: AI-Powered Key Term Extraction**
- Design OpenAI prompt for contract analysis (few-shot examples)
- Extract: parties, dates, amounts, payment terms, renewal info
- Map extracted data to contract_metadata schema
- Implement structured output parsing (JSON validation)
- Add cost tracking for API calls
- **Deliverable**: Extract key terms from 90%+ of test contracts
- **Owner**: 1 Backend engineer + AI prompt engineer
- **Effort**: 55 hours

**Task 2.4: Risk Flagging & Analysis**
- Implement rule-based risk detector:
  - Auto-renewal clauses
  - Payment terms >60 days
  - Missing end dates
  - Unfavorable penalty clauses
- Build AI-powered secondary risk check via OpenAI
- Store flags in risk_flags table
- Create risk severity scoring (high/medium/low)
- **Deliverable**: Flag common risks, display on dashboard
- **Owner**: 1 Backend engineer
- **Effort**: 40 hours

#### Phase 3: Tracking & Notifications (Weeks 8-10)

**Task 3.1: Renewal Calendar & Reminders**
- Build renewal_reminders table logic
- Create calendar view (React Calendar library)
- Implement email reminder service (SendGrid/AWS SES)
- Set up scheduled task (Cloud Tasks) for daily reminder checks
- Send emails: 30 days, 14 days, 7 days before renewal
- **Deliverable**: Users receive email reminders, see calendar
- **Owner**: 1 Backend + 1 Frontend engineer
- **Effort**: 45 hours

**Task 3.2: Dashboard & Contract Overview**
- Build dashboard showing:
  - Total contracts, upcoming renewals (next 30 days)
  - High-risk contracts count
  - Recent uploads
- Contract detail page: metadata, risks, timeline
- Implement filtering and search (by vendor, status, risk level)
- Add CSV export functionality for contract metadata
- **Deliverable**: Executive can scan contracts at a glance
- **Owner**: 1 Frontend + 1 Backend engineer
- **Effort**: 50 hours

**Task 3.3: Team Collaboration Features**
- Implement team member invitations
- Add audit logging (who viewed/downloaded what, when)
- Build activity feed for team actions
- **Deliverable**: Multiple users can collaborate, audit trail exists
- **Owner**: 1 Backend engineer
- **Effort**: 30 hours

#### Phase 4: Monetization & Refinement (Weeks 11-14)

**Task 4.1: Freemium Tier Implementation**
- Implement contract count limits (5 contracts free tier)
- Add payment system integration (Stripe)
- Create subscription management (upgrade/downgrade)
- Implement feature flags for paid features:
  - Risk scoring (paid only)
  - Auto-renewal alerts (paid only)
  - Team collaboration (paid only)
- **Deliverable**: Users hit free limit, see upgrade prompts
- **Owner**: 1 Backend engineer + 1 Frontend engineer
- **Effort**: 55 hours

**Task 4.2: Performance & Scalability Optimization**
- Optimize database queries (add indexes, analyze slow queries)
- Implement caching layer (Redis) for contract metadata
- Optimize Cloud Run cold starts
- Load test with 1000 concurrent users
- **Deliverable**: <2s dashboard load time, <3s extraction
- **Owner**: 1 Backend/DevOps engineer
- **Effort**: 35 hours

**Task 4.3: Security & Compliance Hardening**
- Implement rate limiting on API endpoints
- Add CORS, CSRF protection
- Encrypt sensitive fields (SSN, account numbers) at rest
- Implement data retention policies (soft deletes)
- Add SOC 2 readiness checks (audit logs, encryption)
- **Deliverable**: Security audit passes, no critical issues
- **Owner**: 1 Backend/Security engineer
- **Effort**: 40 hours

#### Phase 5: Launch & Polish (Weeks 15-16)

**Task 5.1: Testing, Documentation & Launch**
- Write integration tests (90%+ API coverage)
- Create user documentation (getting started guide)
- Build help center content
- Prepare analytics dashboard (Google Analytics, custom events)
- Load test in production environment
- Create runbooks for common issues
- **Deliverable**: Ready for beta launch
- **Owner**: QA + 1 Backend engineer
- **Effort**: 50 hours

**Task 5.2: Monitoring, Logging & Incident Response**
- Set up Cloud Logging & Cloud Monitoring
- Create alerts: API errors >1%, DB CPU >80%, quota limits
- Build Slack integration for critical alerts
- Create incident response runbook
- Set up error tracking (Sentry)
- **Deliverable**: Ops team can monitor production
- **Owner**: 1 DevOps engineer
- **Effort**: 30 hours

---

## Summary

| Phase | Duration | Team Size | Key Deliverable |
|-------|----------|-----------|-----------------|
| 1: Foundation | 3 weeks | 2 eng | Auth + local dev ready |
| 2: Core Contract Mgmt | 4 weeks | 3 eng | Extract key terms, flag risks |
| 3: Tracking & Collab | 3 weeks | 3 eng | Renewal calendar, dashboard |
| 4: Monetization | 4 weeks | 3 eng | Freemium tiers, scale ready |
| 5: Launch | 2 weeks | 2 eng | Monitor, docs, live |
| **Total** | **16 weeks** | **3-4 eng** | **MVP Live** |

### Resource Requirements
- **Headcount**: 3-4 engineers (1 backend lead, 1 frontend, 1 DevOps/full-stack, 1 part-time AI/prompt)
- **Infrastructure**: ~$300/month GCP
- **External APIs**: ~$200-500/month (OpenAI @ scale + email service)
- **Tools**: GitHub Pro ($21/month), Stripe account (2.9% + $0.30), SendGrid/SES

### Success Criteria
✅ Users can upload & process 5+ contracts  
✅ Extract accuracy >90% on key terms  
✅ Risk flags prevent >1 missed renewal per user  
✅ 60%+ free→paid conversion  
✅ <2s dashboard load time  
✅ Zero critical security issues  

---

**Document Version**: 1.0  
**Last Updated**: [Current Date]  
**Next Review**: After Phase 2 completion