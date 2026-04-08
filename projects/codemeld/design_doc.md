# CodeMeld - Detailed Design Document

## Overview

CodeMeld is an AI-powered code review automation platform designed to reduce code review latency and enforce architectural consistency across distributed engineering teams. The MVP focuses on GitHub integration, intelligent PR categorization, automated review generation, and review metrics visualization.

### Vision
Enable teams to maintain high code quality standards while eliminating review bottlenecks through intelligent automation and real-time architectural guidance.

### MVP Goals
- Reduce average code review time by 40-60%
- Auto-categorize PRs into priority tiers within 30 seconds of creation
- Generate contextual review summaries covering 80%+ of common issues
- Provide actionable team insights via analytics dashboard
- Maintain <2s P95 latency for all API operations

---

## Tech Stack

### Backend
- **Language**: Python 3.11+
  - Rationale: Optimal for LLM integration (OpenAI SDK, LangChain), ML workflows, and rapid development. Strong async libraries (asyncio, FastAPI) for handling concurrent webhook processing.

- **Framework**: FastAPI
  - Rationale: Modern async framework with automatic OpenAPI documentation, built-in dependency injection, and excellent performance for webhooks and real-time operations.

- **LLM Integration**: OpenAI API (GPT-4 Turbo) + LangChain
  - Rationale: Production-ready, reliable, and supports cost-effective batch processing for summaries. LangChain provides abstraction layer for future model switching.

- **Static Analysis**: 
  - Pylint, ESLint, Prettier (language-specific analyzers)
  - SonarQube Community Edition for baseline architectural analysis
  - Custom rule engine for pattern matching

- **API Client**: PyGithub for GitHub integration
  - Alternative: Use GitHub GraphQL API directly for advanced queries

### Frontend
- **Framework**: React 18 + TypeScript
  - Rationale: Component-based UI, strong typing, and ecosystem maturity for dashboard development.

- **State Management**: TanStack Query (React Query) + Zustand
  - Rationale: TanStack Query handles server state/caching elegantly. Zustand provides lightweight client state without boilerplate.

- **Styling**: Tailwind CSS + shadcn/ui
  - Rationale: Rapid UI development with pre-built accessible components.

- **Visualization**: Recharts for metrics/analytics
  - Rationale: Lightweight, React-native charts without D3 complexity.

### DevOps & Infrastructure
- **Containerization**: Docker + Docker Compose for local development
- **Orchestration**: Kubernetes (EKS on AWS) for production
- **Message Queue**: Redis (for webhook processing) + optional SQS for async jobs
- **Cache**: Redis (shared with queue)
- **Monitoring**: Prometheus + Grafana + ELK Stack
- **Error Tracking**: Sentry

---

## Database Design

### Decision: SQL + NoSQL Hybrid

**SQL (PostgreSQL)** for:
- Transactional data (teams, users, subscriptions)
- PR metadata and review state
- Configuration and rules
- Audit logs

**NoSQL (MongoDB/DynamoDB)** for:
- PR analysis results (JSON-flexible structure)
- LLM-generated content (summaries, suggestions)
- User analytics and events (high-volume writes, denormalized)

### Rationale
- PostgreSQL handles relational requirements and consistency
- MongoDB accommodates flexible LLM output schemas without schema migrations
- Separation allows optimized indexing per workload

### PostgreSQL Schema

```sql
-- Users & Authentication
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    github_id INTEGER UNIQUE,
    github_login VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Teams
CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    github_org_id INTEGER UNIQUE,
    github_org_name VARCHAR(255),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    tier VARCHAR(50) DEFAULT 'free', -- free, professional, enterprise
    member_limit INTEGER DEFAULT 5,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_owner_id (owner_id)
);

-- Team Members
CREATE TABLE team_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) DEFAULT 'member', -- owner, admin, member
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(team_id, user_id),
    INDEX idx_team_id (team_id)
);

-- GitHub Repositories (linked to teams)
CREATE TABLE repositories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    github_repo_id INTEGER UNIQUE NOT NULL,
    github_repo_name VARCHAR(255) NOT NULL,
    github_owner VARCHAR(255) NOT NULL,
    full_name VARCHAR(511) NOT NULL, -- owner/repo
    is_active BOOLEAN DEFAULT TRUE,
    webhook_id INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_team_id (team_id),
    INDEX idx_full_name (full_name)
);

-- Pull Requests
CREATE TABLE pull_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    github_pr_id INTEGER NOT NULL,
    github_pr_number INTEGER NOT NULL,
    title VARCHAR(511) NOT NULL,
    author_github_login VARCHAR(255),
    status VARCHAR(50) DEFAULT 'open', -- open, closed, merged
    priority VARCHAR(50) DEFAULT 'medium', -- critical, high, medium, low
    complexity VARCHAR(50) DEFAULT 'medium', -- trivial, small, medium, large
    lines_added INTEGER DEFAULT 0,
    lines_deleted INTEGER DEFAULT 0,
    files_changed INTEGER DEFAULT 0,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    merged_at TIMESTAMP,
    review_requested_at TIMESTAMP,
    INDEX idx_repository_id (repository_id),
    INDEX idx_status (status),
    INDEX idx_priority (priority),
    INDEX idx_created_at (created_at)
);

-- Architectural Rules (team-specific)
CREATE TABLE architectural_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    rule_type VARCHAR(50), -- pattern, naming, structure, dependency
    pattern VARCHAR(1000), -- regex or pattern string
    severity VARCHAR(50) DEFAULT 'warning', -- error, warning, info
    is_active BOOLEAN DEFAULT TRUE,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_team_id (team_id)
);

-- Review Metrics (aggregated, denormalized for performance)
CREATE TABLE review_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    total_prs INTEGER DEFAULT 0,
    avg_review_time_hours DECIMAL(10, 2),
    critical_prs_count INTEGER DEFAULT 0,
    avg_lines_changed INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(team_id, date),
    INDEX idx_team_id (team_id)
);

-- Subscription & Billing
CREATE TABLE subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE RESTRICT,
    tier VARCHAR(50) NOT NULL, -- free, professional, enterprise
    stripe_customer_id VARCHAR(255),
    stripe_subscription_id VARCHAR(255),
    billing_email VARCHAR(255),
    status VARCHAR(50) DEFAULT 'active', -- active, past_due, canceled
    current_period_start DATE,
    current_period_end DATE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_team_id (team_id)
);

-- Audit Logs
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(255) NOT NULL,
    resource_type VARCHAR(100),
    resource_id VARCHAR(255),
    changes JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_team_id (team_id),
    INDEX idx_created_at (created_at)
);
```

### MongoDB Collections

```javascript
// PR Analysis Results
db.createCollection("pr_analyses", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            required: ["pr_id", "team_id", "analysis_data"],
            properties: {
                _id: { bsonType: "objectId" },
                pr_id: { bsonType: "string" }, // UUID from PostgreSQL
                team_id: { bsonType: "string" },
                repository_id: { bsonType: "string" },
                analysis_data: {
                    bsonType: "object",
                    properties: {
                        files_analyzed: { bsonType: "int" },
                        issues_detected: { bsonType: "array" },
                        pattern_violations: { bsonType: "array" },
                        security_flags: { bsonType: "array" },
                        code_quality_score: { bsonType: "double" },
                        languages: { bsonType: "array" }
                    }
                },
                created_at: { bsonType: "date" },
                updated_at: { bsonType: "date" }
            }
        }
    }
});

// LLM-Generated Summaries & Suggestions
db.createCollection("review_summaries", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            required: ["pr_id", "team_id"],
            properties: {
                _id: { bsonType: "objectId" },
                pr_id: { bsonType: "string" },
                team_id: { bsonType: "string" },
                summary: { bsonType: "string" },
                key_changes: { bsonType: "array" },
                suggested_reviewers: { bsonType: "array" },
                potential_issues: { bsonType: "array" },
                architectural_concerns: { bsonType: "array" },
                confidence_score: { bsonType: "double" },
                model_version: { bsonType: "string" },
                generated_at: { bsonType: "date" },
                tokens_used: { bsonType: "int" }
            }
        }
    }
});

// Team Analytics Events (high-volume)
db.createCollection("analytics_events", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            properties: {
                _id: { bsonType: "objectId" },
                team_id: { bsonType: "string" },
                event_type: { bsonType: "string" },
                event_data: { bsonType: "object" },
                timestamp: { bsonType: "date" }
            }
        }
    }
});

// Create indexes
db.pr_analyses.createIndex({ pr_id: 1, team_id: 1 });
db.pr_analyses.createIndex({ team_id: 1, created_at: -1 });
db.review_summaries.createIndex({ pr_id: 1 });
db.review_summaries.createIndex({ team_id: 1, generated_at: -1 });
db.analytics_events.createIndex({ team_id: 1, timestamp: -1 });
db.analytics_events.createIndex({ event_type: 1, timestamp: -1 });
```

---

## Storage Plan

### Data Storage Strategy

| Data Type | Storage | Rationale |
|-----------|---------|-----------|
| **PR Diffs & Code** | GitHub (external) | No local storage; fetch on-demand via GitHub API |
| **Analysis Results** | MongoDB | Flexible schema for diverse analysis outputs |
| **LLM Outputs** | MongoDB + S3 | Cache summaries in Mongo; archive to S3 after 90 days for compliance |
| **User/Team Data** | PostgreSQL | ACID guarantees required |
| **User-Uploaded Rules** | PostgreSQL (rules table) + S3 | Small rules in DB; large config files in S3 |
| **Analytics/Events** | MongoDB (hot) + Data Warehouse | Recent data in Mongo; aggregate to DW weekly |
| **Logs & Traces** | ELK Stack + S3 | Real-time in ELK; archive to S3 after 30 days |
| **Session/Cache** | Redis | Ephemeral, no backup required |

### S3 Bucket Structure (AWS)
```
codemeld-prod/
├── pr-artifacts/
│   ├── {team_id}/{pr_id}/diff.patch
│   ├── {team_id}/{pr_id}/analysis_report.json
├── archived-summaries/
│   ├── 2024-01/{team_id}/{pr_id}.json
├── user-rules/
│   ├── {team_id}/rules-backup.yaml
├── logs/
│   ├── 2024-01-15/codemeld-api-logs.gz
└── compliance/
    ├── audit-trails-encrypted/
    └── user-data-exports/
```

### Retention Policies
- **PR Analysis Data**: 90 days in MongoDB, then archive to S3
- **Review Summaries**: Indefinite in MongoDB (low volume)
- **Audit Logs**: 7 years in PostgreSQL (compliance requirement)
- **Analytics Events**: 12 months aggregated; raw events 90 days
- **Application Logs**: 30 days in ELK; 1 year in S3 (encrypted)

### Data Growth Projections (per 100 teams)
- **PostgreSQL**: ~50GB/year (users, repos, PRs, rules)
- **MongoDB**: ~200GB/year (analyses, summaries, events)
- **S3**: ~100GB/year (archived data)
- **Redis**: ~5GB (ephemeral)

---

## Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        GitHub/GitLab                         │
│                    (External Data Source)                    │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────┐
│                   API Gateway & Auth Service                 │
│  (FastAPI + OAuth 2.0/GitHub Apps + Rate Limiting)          │
└──────────────────────────────────────────────────────────────┘
       ↓                      ↓                      ↓
┌──────────────┐  ┌──────────────────┐  ┌────────────────────┐
│ Webhook      │  │ REST API Service │  │ Analytics Service  │
│ Receiver     │  │ (PR endpoints,   │  │ (Dashboard data,   │
│ (async queue)│  │  rule mgmt, team)│  │  metrics, trends)  │
└──────────────┘  └──────────────────┘  └────────────────────┘
       ↓                      ↓                      ↓┌──────────────────────────────────────────────────────────┐
│         PR Processing Pipeline (Async Workers)              │
├──────────────────────────────────────────────────────────────┤
│  1. Code Analysis Worker    (Static analysis, linting)      │
│  2. Pattern Matching Worker (Architectural rule checking)   │
│  3. LLM Worker             (Summary generation, suggestions)│
│  4. Priority/Complexity Worker (ML-based categorization)   │
└──────────────────────────────────────────────────────────────┘
       ↓              ↓              ↓              ↓
┌──────────────────────────────────────────────────────────────┐
│             Data Storage Layer                              │
├──────────────────────────────────────────────────────────────┤
│  PostgreSQL (transactional) │ MongoDB (analyses)            │
│  Redis (cache/queue)         │ S3 (archives)                │
└──────────────────────────────────────────────────────────────┘
       ↓                                           ↓
┌──────────────────────┐              ┌────────────────────────┐
│ Frontend Dashboard   │              │ Notification Service   │
│ (React + TypeScript) │              │ (Slack, Email)        │
└──────────────────────┘              └────────────────────────┘
```

### Core Services Architecture

#### 1. **API Service** (FastAPI Application)
```python
# app/main.py structure
app/
├── main.py                 # FastAPI app initialization
├── config.py              # Environment config, secrets
├── middleware/
│   ├── auth.py           # GitHub OAuth + JWT verification
│   ├── rate_limit.py     # Per-team rate limiting
│   └── error_handler.py  # Global exception handling
├── api/
│   ├── v1/
│   │   ├── auth.py       # Login, logout, token refresh
│   │   ├── teams.py      # Team CRUD, member management
│   │   ├── repositories.py # Repo linking, webhook setup
│   │   ├── pull_requests.py # PR queries, status updates
│   │   ├── rules.py      # Architectural rule CRUD
│   │   └── metrics.py    # Dashboard data, analytics
│   └── webhook.py        # GitHub webhook receiver
├── services/
│   ├── github_client.py   # GitHub API wrapper
│   ├── analysis_service.py # Orchestrates PR analysis
│   ├── llm_service.py     # LLM calls via LangChain
│   └── notification_service.py # Slack/Email notifications
├── workers/
│   ├── code_analyzer.py   # Static analysis jobs
│   ├── pattern_matcher.py # Rule enforcement
│   ├── llm_summarizer.py  # Summary generation
│   └── priority_classifier.py # ML categorization
├── models/
│   ├── database.py        # SQLAlchemy ORM models
│   ├── schemas.py         # Pydantic request/response schemas
│   └── mongo_models.py    # MongoEngine schemas
└── utils/
    ├── cache.py          # Redis operations
    ├── logging.py        # Structured logging
    └── validators.py     # Input validation
```

#### 2. **Webhook Processing Flow**
```
GitHub PR Event → API Gateway
                     ↓
              Webhook Receiver
                     ↓
         Validate & Parse Event
                     ↓
      Enqueue to Redis (Celery/RQ)
                     ↓
    ┌─────────────────┬──────────────┬──────────────┐
    ↓                 ↓              ↓              ↓
Code Analyzer    Pattern Matcher  LLM Worker   Priority Worker
(1-2s)          (500ms)          (3-5s)       (1-2s)
    ↓                 ↓              ↓              ↓
    └─────────────────┴──────────────┴──────────────┘
                     ↓
         Aggregate Results
                     ↓
      Store in PostgreSQL + MongoDB
                     ↓
      Notify Team (Slack/Email)
                     ↓
    Update Frontend (WebSocket/Polling)
```

#### 3. **LLM Integration Pattern** (LangChain)
```python
# services/llm_service.py
from langchain.chat_models import ChatOpenAI
from langchain.prompts import ChatPromptTemplate
from langchain.output_parsers import PydanticOutputParser

class LLMReviewService:
    def __init__(self):
        self.llm = ChatOpenAI(
            model="gpt-4-turbo-preview",
            temperature=0.2,  # Low temperature for consistency
            max_tokens=2000,
            request_timeout=30
        )
    
    async def generate_summary(self, pr_diff: str, team_context: dict):
        """Generate review summary with caching."""
        cache_key = f"summary:{pr_id}"
        cached = await redis.get(cache_key)
        if cached:
            return json.loads(cached)
        
        prompt = ChatPromptTemplate.from_template("""
        Review this PR diff and provide:
        1. Executive summary (2-3 sentences)
        2. Key changes (3-5 bullet points)
        3. Potential issues or concerns
        4. Architectural impact
        
        Team coding standards: {standards}
        Previous PRs context: {context}
        
        PR Diff:
        {diff}
        """)
        
        parser = PydanticOutputParser(
            pydantic_object=ReviewSummaryOutput
        )
        
        chain = prompt | self.llm | parser
        result = await chain.ainvoke({
            "standards": team_context.get("standards"),
            "context": team_context.get("recent_prs"),
            "diff": pr_diff
        })
        
        # Cache for 7 days
        await redis.setex(cache_key, 604800, json.dumps(result.dict()))
        return result
```

#### 4. **ML-Based Priority Classification**
```python
# workers/priority_classifier.py
import joblib
import numpy as np

class PriorityClassifier:
    def __init__(self):
        self.model = joblib.load("models/priority_classifier.pkl")
        self.scaler = joblib.load("models/feature_scaler.pkl")
    
    def predict_priority(self, pr_features: dict) -> tuple[str, float]:
        """
        Returns: (priority_level, confidence)
        priority_level: critical, high, medium, low
        """
        features = np.array([
            pr_features["lines_changed"],
            pr_features["files_changed"],
            pr_features["num_comments"],
            pr_features["author_experience_level"],
            pr_features["mentions_critical_paths"],
            pr_features["security_flags_count"],
            pr_features["test_coverage_change"],
            pr_features["hour_of_day"],
        ]).reshape(1, -1)
        
        scaled_features = self.scaler.transform(features)
        
        # Get prediction and confidence
        prediction = self.model.predict(scaled_features)[0]
        probability = self.model.predict_proba(scaled_features).max()
        
        priority_map = {0: "critical", 1: "high", 2: "medium", 3: "low"}
        return priority_map[prediction], float(probability)
    
    def learn_from_team_patterns(self, team_id: str):
        """Fine-tune model with team's historical PR data."""
        # Fetch team's last 500 PRs with review times
        historical_data = fetch_team_pr_history(team_id)
        # Retrain on team-specific patterns (batch job, weekly)
        # Store team-specific model variant
```

#### 5. **Pattern Matching Engine** (Architectural Rules)
```python
# workers/pattern_matcher.py
import re
from typing import List

class PatternMatcher:
    def __init__(self, team_id: str):
        self.team_id = team_id
        self.rules = self._load_team_rules()
    
    def _load_team_rules(self) -> List[dict]:
        """Load active rules for team from PostgreSQL."""
        return db.query(ArchitecturalRule).filter(
            ArchitecturalRule.team_id == self.team_id,
            ArchitecturalRule.is_active == True
        ).all()
    
    async def check_violations(self, pr_diff: str, files_changed: List[str]):
        """Check PR against all team rules."""
        violations = []
        
        for rule in self.rules:
            if rule.rule_type == "pattern":
                matches = re.findall(rule.pattern, pr_diff)
                if matches:
                    violations.append({
                        "rule_id": rule.id,
                        "rule_name": rule.name,
                        "severity": rule.severity,
                        "matches_count": len(matches),
                        "description": rule.description
                    })
            
            elif rule.rule_type == "naming":
                # Check file/function naming conventions
                await self._check_naming_convention(rule, files_changed)
            
            elif rule.rule_type == "dependency":
                # Check for forbidden/required dependencies
                await self._check_dependencies(rule, pr_diff)
            
            elif rule.rule_type == "structure":
                # Check directory structure changes
                await self._check_structure(rule, files_changed)
        
        return violations
```

---

## Deployment Plan

### Containerization Strategy

#### Docker Images

```dockerfile
# Dockerfile.api
FROM python:3.11-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy requirements
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application
COPY ./app ./app

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Run with gunicorn
CMD ["gunicorn", "app.main:app", \
     "--workers=4", \
     "--worker-class=uvicorn.workers.UvicornWorker", \
     "--bind=0.0.0.0:8000", \
     "--access-logfile=-", \
     "--error-logfile=-"]
```

```dockerfile
# Dockerfile.worker
FROM python:3.11-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    git curl gcc \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY ./app ./app

# Worker startup script
COPY docker-entrypoint-worker.sh .
RUN chmod +x docker-entrypoint-worker.sh

CMD ["./docker-entrypoint-worker.sh"]
```

```dockerfile
# Dockerfile.frontend
FROM node:18-alpine AS builder

WORKDIR /app
COPY package*.json ./
RUN npm ci

COPY . .
RUN npm run build

# Production stage
FROM nginx:alpine

COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

#### Docker Compose (Development)

```yaml
# docker-compose.yml
version: '3.8'

services:
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_DB: codemeld
      POSTGRES_USER: codemeld
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U codemeld"]
      interval: 10s
      timeout: 5s
      retries: 5

  mongo:
    image: mongo:6.0
    environment:
      MONGO_INITDB_ROOT_USERNAME: codemeld
      MONGO_INITDB_ROOT_PASSWORD: ${MONGO_PASSWORD}
    volumes:
      - mongo_data:/data/db
    ports:
      - "27017:27017"
    healthcheck:
      test: echo 'db.runCommand("ping").ok' | mongosh localhost:27017/test --quiet
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  api:
    build:
      context: .
      dockerfile: Dockerfile.api
    environment:
      DATABASE_URL: postgresql://codemeld:${DB_PASSWORD}@postgres:5432/codemeld
      MONGO_URI: mongodb://codemeld:${MONGO_PASSWORD}@mongo:27017/codemeld?authSource=admin
      REDIS_URL: redis://redis:6379
      GITHUB_CLIENT_ID: ${GITHUB_CLIENT_ID}
      GITHUB_CLIENT_SECRET: ${GITHUB_CLIENT_SECRET}
      OPENAI_API_KEY: ${OPENAI_API_KEY}
    ports:
      - "8000:8000"
    depends_on:
      postgres:
        condition: service_healthy
      mongo:
        condition: service_healthy
      redis:
        condition: service_healthy
    volumes:
      - ./app:/app
    command: uvicorn app.main:app --host 0.0.0.0 --reload

  worker:
    build:
      context: .
      dockerfile: Dockerfile.worker
    environment:
      DATABASE_URL: postgresql://codemeld:${DB_PASSWORD}@postgres:5432/codemeld
      MONGO_URI: mongodb://codemeld:${MONGO_PASSWORD}@mongo:27017/codemeld?authSource=admin
      REDIS_URL: redis://redis:6379
      OPENAI_API_KEY: ${OPENAI_API_KEY}
    depends_on:
      - postgres
      - mongo
      - redis
    volumes:
      - ./app:/app
    deploy:
      replicas: 2

  frontend:
    build:
      context: ./frontend
      dockerfile: Dockerfile.frontend
    ports:
      - "3000:80"
    depends_on:
      - api

volumes:
  postgres_data:
  mongo_data:
```

### CI/CD Pipeline

#### GitHub Actions Workflow

```yaml
# .github/workflows/ci-cd.yml
name: CodeMeld CI/CD

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  test-backend:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15-alpine
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      redis:
        image: redis:7-alpine
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'
          cache: 'pip'

      - name: Install dependencies
        run: |
          python -m pip install --upgrade pip
          pip install -r requirements.txt
          pip install pytest pytest-cov pytest-asyncio

      - name: Run linting
        run: |
          pylint app/ --fail-under=8.0
          black --check app/
          isort --check-only app/

      - name: Run unit tests
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/codemeld_test
          REDIS_URL: redis://localhost:6379
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          pytest tests/ \
            --cov=app \
            --cov-report=xml \
            --cov-report=term \
            --cov-fail-under=75

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          file: ./coverage.xml

  test-frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Node
        uses: actions/setup-node@v3
        with:
          node-version: '18'
          cache: 'npm'
          cache-dependency-path: frontend/package-lock.json

      - name: Install dependencies
        working-directory: frontend
        run: npm ci

      - name: Run linting
        working-directory: frontend
        run: npm run lint

      - name: Run tests
        working-directory: frontend
        run: npm run test -- --coverage

      - name: Build
        working-directory: frontend
        run: npm run build

  build-and-push:
    needs: [test-backend, test-frontend]
    runs-on: ubuntu-latest
    if: github.event_name == 'push'
    permissions:
      contents: read
      packages: write

    steps:
      - uses: actions/checkout@v3

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v2

      - name: Log in to Container Registry
        uses: docker/login-action@v2
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v4
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=ref,event=branch
            type=semver,pattern={{version}}
            type=sha

      - name: Build and push API image
        uses: docker/build-push-action@v4
        with:
          context: .
          file: ./Dockerfile.api
          push: true
          tags: ${{ steps.meta.outputs.tags }}-api
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=registry,ref=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:buildcache-api
          cache-to: type=registry,ref=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:buildcache-api,mode=max

      - name: Build and push Worker image
        uses: docker/build-push-action@v4
        with:
          context: .
          file: ./Dockerfile.worker
          push: true
          tags: ${{ steps.meta.outputs.tags }}-worker
          labels: ${{ steps.meta.outputs.labels }}

      - name: Build and push Frontend image
        uses: docker/build-push-action@v4
        with:
          context: ./frontend
          file: ./frontend/Dockerfile.frontend
          push: true
          tags: ${{ steps.meta.outputs.tags }}-frontend
          labels: ${{ steps.meta.outputs.labels }}

  security-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run Trivy vulnerability scanner
        uses: aquasecurity/trivy-action@master
        with:
          scan-type: 'fs'
          scan-ref: '.'
          format: 'sarif'
          output: 'trivy-results.sarif'

      - name: Upload Trivy results
        uses: github/codeql-action/upload-sarif@v2
        with:
          sarif_file: 'trivy-results.sarif'

  deploy-staging:
    needs: [build-and-push, security-scan]
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/develop'
    environment:
      name: staging

    steps:
      - uses: actions/checkout@v3

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v2
        with:
          role-to-assume: arn:aws:iam::${{ secrets.AWS_ACCOUNT_ID }}:role/github-actions
          aws-region: us-east-1

      - name: Update EKS cluster kubeconfig
        run: |
          aws eks update-kubeconfig \
            --name codemeld-staging \
            --region us-east-1

      - name: Deploy to Staging
        run: |
          kubectl set image deployment/codemeld-api \
            api=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}-api \
            -n codemeld
          kubectl set image deployment/codemeld-worker \
            worker=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}-worker \
            -n codemeld
          kubectl set image deployment/codemeld-frontend \
            frontend=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}-frontend \
            -n codemeld
          kubectl rollout status deployment/codemeld-api -n codemeld

      - name: Run smoke tests
        run: |
          ./scripts/smoke-tests.sh staging

  deploy-production:
    needs: [build-and-push, security-scan]
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    environment:
      name: production

    steps:
      - uses: actions/checkout@v3

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v2
        with:
          role-to-assume: arn:aws:iam::${{ secrets.AWS_ACCOUNT_ID }}:role/github-actions
          aws-region: us-east-1

      - name: Update EKS cluster kubeconfig
        run: |
          aws eks update-kubeconfig \
            --name codemeld-production \
            --region us-east-1

      - name: Deploy with canary strategy
        run: |
          kubectl set image deployment/codemeld-api \
            api=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}-api \
            -n codemeld \
            --record
          kubectl rollout status deployment/codemeld-api -n codemeld --timeout=5m

      - name: Run production smoke tests
        run: ./scripts/smoke-tests.sh production

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            CHANGELOG.md
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

---

## Infrastructure

### Cloud Provider: AWS (Primary)

#### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    AWS Account (us-east-1)                │
├─────────────────────────────────────────────────────────────┤
│  CloudFront CDN (Frontend Distribution)                     │
│  Route 53 (DNS)                                             │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│  Application Load Balancer (ALB)                            │
│  - SSL/TLS Termination                                      │
│  - Path-based routing                                       │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│         EKS Cluster (Kubernetes)                            │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐
│  │ API Pods       │  │ Worker Pods    │  │ Frontend Pods  │
│  │ (3-10 replicas)│  │ (5-20 replicas)│  │ (2-5 replicas) │
│  └────────────────┘  └────────────────┘  └────────────────┘
│  HPA: CPU 70%, Memory 80%                                   │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────┬──────────────┬──────────────┬────────────────┐
│              │              │              │                │
↓              ↓              ↓              ↓                ↓
RDS         RDS Aurora   ElastiCache     S3               VPC
PostgreSQL  Read Replica  Redis Cluster  (Data Lake)    (Private)
(Primary)   (standby)     (2 nodes)      (Versioning,
                                         Encryption)
                          
┌─────────────────────────────────────────────────────────────┐
│  Data Layer Security                                        │
│  - VPC Endpoints                                            │
│  - Encryption at rest (KMS)                                 │
│  - Encryption in transit (TLS)                              │
│  - RDS Multi-AZ                                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Monitoring & Logging                                       │
│  - CloudWatch (metrics, logs)                               │
│  - ELK Stack (application logs)                             │
│  - Prometheus + Grafana (in-cluster)                        │
│  - Sentry (error tracking)                                  │
└─────────────────────────────────────────────────────────────┘
```

### AWS Resource Specifications

#### EKS Cluster Configuration

```yaml
# terraform/eks.tf
resource "aws_eks_cluster" "codemeld" {
  name            = "codemeld-${var.environment}"
  role_arn        = aws_iam_role.eks_cluster.arn
  version         = "1.28"

  vpc_config {
    subnet_ids              = aws_subnet.private[*].id
    endpoint_private_access = true
    endpoint_public_access  = true
    security_groups         = [aws_security_group.eks.id]
  }

  enabled_cluster_log_types = [
    "api",
    "audit",
    "authenticator",
    "controllerManager",
    "scheduler"
  ]

  depends_on = [aws_iam_role_policy_attachment.eks_cluster_policy]
}

resource "aws_eks_node_group" "codemeld" {
  cluster_name    = aws_eks_cluster.codemeld.name
  node_group_name = "codemeld-nodes-${var.environment}"
  node_role_arn   = aws_iam_role.eks_node.arn
  subnet_ids      = aws_subnet.private[*].id

  scaling_config {
    desired_size = var.desired_nodes
    max_size     = var.max_nodes
    min_size     = var.min_nodes
  }

  instance_types = ["t3.xlarge"] # Production: r6i.2xlarge

  disk_size = 100

  tags = {
    Name = "codemeld-nodes-${var.environment}"
  }

  depends_on = [
    aws_iam_role_policy_attachment.eks_worker_node_policy,
    aws_iam_role_policy_attachment.eks_cni_policy,
    aws_iam_role_policy_attachment.eks_container_registry_policy,
  ]
}

# Auto Scaling based on metrics
resource "kubernetes_horizontal_pod_autoscaler_v2" "api" {
  metadata {
    name      = "codemeld-api-hpa"
    namespace = kubernetes_namespace.codemeld.metadata[0].name
  }

  spec {
    scale_target_ref {
      api_version = "apps/v1"
      kind        = "Deployment"
      name        = "codemeld-api"
    }

    min_replicas = 3
    max_replicas = 10

    metric {
      type = "Resource"
      resource {
        name = "cpu"
        target {
          type                = "Utilization"
          average_utilization = "70"
        }
      }
    }

    metric {
      type = "Resource"
      resource {
        name = "memory"
        target {
          type                = "Utilization"
          average_utilization = "80"
        }
      }
    }
  }
}
```

#### RDS PostgreSQL

```yaml
# terraform/rds.tf
resource "aws_rds_cluster" "codemeld" {
  cluster_identifier              = "codemeld-${var.environment}"
  engine                          = "aurora-postgresql"
  engine_version                  = "15.2"
  database_name                   = "codemeld"
  master_username                 = var.db_username
  master_password                 = random_password.db_password.result
  
  db_subnet_group_name            = aws_db_subnet_group.codemeld.name
  vpc_security_group_ids          = [aws_security_group.rds.id]
  
  backup_retention_period         = 30
  preferred_backup_window         = "03:00-04:00"
  preferred_maintenance_window    = "sun:04:00-sun:05:00"
  
  enabled_cloudwatch_logs_exports = ["postgresql"]
  
  storage_encrypted               = true
  kms_key_id                      = aws_kms_key.rds.arn
  
  skip_final_snapshot             = false
  final_snapshot_identifier       = "codemeld-${var.environment}-final-${formatdate("YYYY-MM-DD-hhmm", timestamp())}"
  
  # Multi-AZ
  availability_zones              = ["us-east-1a", "us-east-1b", "us-east-1c"]
}

resource "aws_rds_cluster_instance" "primary" {
  cluster_identifier = aws_rds_cluster.codemeld.id
  instance_class     = "db.r6i.xlarge" # Production
  engine              = aws_rds_cluster.codemeld.engine
  engine_version      = aws_rds_cluster.codemeld.engine_version

  monitoring_interval = 60
  monitoring_role_arn = aws_iam_role.rds_monitoring.arn

  performance_insights_enabled    = true
  performance_insights_kms_key_id = aws_kms_key.rds.arn
}

resource "aws_rds_cluster_instance" "read_replica" {
  count              = var.environment == "production" ?2 : 0
  cluster_identifier = aws_rds_cluster.codemeld.id
  instance_class     = "db.r6i.large"
  engine              = aws_rds_cluster.codemeld.engine
  engine_version      = aws_rds_cluster.codemeld.engine_version
}
```

#### ElastiCache Redis

```yaml
# terraform/elasticache.tf
resource "aws_elasticache_replication_group" "codemeld" {
  replication_group_description = "CodeMeld Redis Cluster"
  engine                        = "redis"
  engine_version                = "7.0"
  node_type                      = "cache.r6g.xlarge" # Production
  num_cache_clusters            = 2
  parameter_group_name          = aws_elasticache_parameter_group.codemeld.name
  port                          = 6379
  
  subnet_group_name             = aws_elasticache_subnet_group.codemeld.name
  security_group_ids            = [aws_security_group.redis.id]
  
  automatic_failover_enabled    = true
  multi_az_enabled              = true
  
  at_rest_encryption_enabled    = true
  transit_encryption_enabled    = true
  auth_token                    = random_password.redis_auth.result
  
  snapshot_retention_limit      = 5
  snapshot_window               = "03:00-05:00"
  maintenance_window            = "sun:05:00-sun:07:00"
  
  log_delivery_configuration {
    destination      = aws_cloudwatch_log_group.redis_slow.name
    destination_type = "cloudwatch-logs"
    log_format       = "json"
    log_type         = "slow-log"
  }

  notification_topic_arn        = aws_sns_topic.alerts.arn

  tags = {
    Name = "codemeld-redis-${var.environment}"
  }
}

resource "aws_elasticache_parameter_group" "codemeld" {
  family = "redis7"
  name   = "codemeld-${var.environment}"

  parameter {
    name  = "maxmemory-policy"
    value = "allkeys-lru"
  }

  parameter {
    name  = "timeout"
    value = "300"
  }
}
```

#### S3 Data Lake

```yaml
# terraform/s3.tf
resource "aws_s3_bucket" "codemeld_data" {
  bucket = "codemeld-data-${data.aws_caller_identity.current.account_id}"
}

resource "aws_s3_bucket_versioning" "codemeld_data" {
  bucket = aws_s3_bucket.codemeld_data.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "codemeld_data" {
  bucket = aws_s3_bucket.codemeld_data.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = aws_kms_key.s3.arn
    }
  }
}

resource "aws_s3_bucket_public_access_block" "codemeld_data" {
  bucket = aws_s3_bucket.codemeld_data.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_lifecycle_configuration" "codemeld_data" {
  bucket = aws_s3_bucket.codemeld_data.id

  rule {
    id     = "archive-old-pr-artifacts"
    status = "Enabled"

    filter {
      prefix = "pr-artifacts/"
    }

    transition {
      days          = 90
      storage_class = "GLACIER"
    }

    expiration {
      days = 2555 # 7 years for compliance
    }
  }

  rule {
    id     = "archive-logs"
    status = "Enabled"

    filter {
      prefix = "logs/"
    }

    transition {
      days          = 30
      storage_class = "GLACIER"
    }
  }
}
```

---

## Cost Estimate

### Monthly Cost Breakdown (Production Environment)

#### Compute Costs

| Service | Instance Type | Quantity | Monthly Cost |
|---------|---------------|----------|--------------|
| **EKS Cluster** | | | |
| - API Nodes | t3.xlarge | 5 avg | $600 |
| - Worker Nodes | t3.xlarge | 8 avg | $960 |
| - Frontend Nodes | t3.large | 3 avg | $270 |
| EKS Cluster Management | | 1 | $73 |
| **Total Compute** | | | **$1,903** |

#### Database Costs

| Service | Instance Type | Qty | Monthly Cost |
|---------|---------------|-----|--------------|
| **RDS Aurora PostgreSQL** | | | |
| - Primary | db.r6i.xlarge | 1 | $1,200 |
| - Read Replicas | db.r6i.large | 2 | $1,200 |
| - Storage (500GB) | | | $125 |
| - Backups | | | $50 |
| **ElastiCache Redis** | | | |
| - Cluster nodes | cache.r6g.xlarge | 2 | $800 |
| - Backup storage | | | $75 |
| **MongoDB Atlas** (SaaS) | M30 cluster | 1 | $400 |
| **Total Database** | | | **$3,850** |

#### Storage & Data Transfer

| Service | Specification | Monthly Cost |
|---------|---------------|--------------|
| S3 Standard | 500GB/month | $11 |
| S3 Glacier | 5TB archived | $25 |
| Data Transfer (out) | 100GB/month | $9 |
| **Total Storage** | | **$45** |

#### Networking

| Service | Specification | Monthly Cost |
|---------|---------------|--------------|
| NAT Gateway | 2 gateways | $64 |
| Application Load Balancer | 1 ALB | $16 |
| Data transfer (inter-AZ) | ~50GB | $5 |
| CloudFront CDN | 1TB/month | $85 |
| **Total Networking** | | **$170** |

#### Third-Party Services

| Service | Plan | Monthly Cost |
|---------|------|--------------|
| OpenAI API | Pay-as-you-go | ~$500* |
| GitHub Enterprise | 100 users | $231 |
| Sentry | Professional | $99 |
| Auth0 | Professional | $149 |
| Stripe (payment processing) | 2.9% + $0.30 | ~$100** |
| SendGrid (email) | 100K/month free tier | $0 |
| **Total Third-Party** | | **$1,079** |

#### Monitoring & Logging

| Service | Specification | Monthly Cost |
|---------|---------------|--------------|
| CloudWatch Logs | 10GB/month | $50 |
| CloudWatch Metrics | 1000 metrics | $100 |
| Prometheus/Grafana (self-hosted) | Included in compute | $0 |
| ELK Stack (self-hosted, t3.large) | 1 instance | $90 |
| **Total Monitoring** | | **$240** |

#### Security & Compliance

| Service | Specification | Monthly Cost |
|---------|---------------|--------------|
| AWS Certificate Manager | Unlimited certs | $0 |
| KMS Keys | 3 keys | $3 |
| VPC & SecurityGroups | | $0 |
| **Total Security** | | **$3** |

### Summary

```
Compute:              $1,903
Database:             $3,850
Storage/Data:            $45
Networking:             $170
Third-Party:          $1,079
Monitoring:             $240
Security:               $3
────────────────────────────
SUBTOTAL:             $7,290

Buffer (15%):         $1,093
────────────────────────────
TOTAL MONTHLY:        $8,383
```

**Annual Cost**: ~$100,596

### Cost Optimization Opportunities

1. **Reserved Instances** (30-40% savings): Commit to 1-3 year reservations for consistent workloads
   - Potential savings: ~$2,000/month
   
2. **Spot Instances** for worker nodes: Use for non-critical async jobs
   - Potential savings: ~$400/month
   
3. **MongoDB Atlas downgrade**: Switch to M10 after MVP stabilizes
   - Potential savings: ~$250/month
   
4. **OpenAI batch processing**: Use cheaper batch API for summary generation
   - Potential savings: ~$200/month (off-peak hours)

**Optimized Monthly Cost: ~$5,500** (~$66,000/year)

### Scaling Cost Projections

| Metric | Month 1 | Month 6 | Year 1 |
|--------|---------|---------|--------|
| Teams | 10 | 100 | 500 |
| Database Storage | 20GB | 150GB | 400GB |
| LLM Requests/day | 1,000 | 50,000 | 250,000 |
| Data Transfer/month | 10GB | 200GB | 500GB |
| **Est. Monthly Cost** | $5,500 | $7,200 | $12,000 |

---

## Task Breakdown - MVP Engineering Tasks

### Task 1: Core API Service & Authentication (2 weeks)

**Objective**: Establish FastAPI application foundation with GitHub OAuth integration and team/user management

**Deliverables**:
- FastAPI application structure with middleware (auth, logging, error handling)
- PostgreSQL ORM models for users, teams, repositories
- GitHub OAuth 2.0 flow implementation
- JWT token generation and validation
- Team creation and member management endpoints
- Database migrations with Alembic
- Docker image and local development environment

**Technical Details**:
```python
# Core endpoints to implement
POST   /api/v1/auth/login
POST   /api/v1/auth/logout
POST   /api/v1/auth/refresh
GET    /api/v1/teams
POST   /api/v1/teams
GET    /api/v1/teams/{id}/members
POST   /api/v1/teams/{id}/members
```

**Dependencies**: PostgreSQL setup, GitHub app registration
**Acceptance Criteria**:
- Users can log in via GitHub OAuth
- Teams can be created and members added
- JWT tokens validated on protected endpoints
- Unit tests for auth flows (>80% coverage)

---

### Task 2: GitHub Integration & Webhook Processing (2 weeks)

**Objective**: Implement GitHub API integration and webhook receiver for PR events

**Deliverables**:
- GitHub API wrapper using PyGithub (repository linking, PR fetching)
- Webhook receiver endpoint with signature validation
- Webhook event parser and routing logic
- Redis-based message queue setup with Celery/RQ
- Async job enqueueing for PR analysis
- Repository connection flow in API
- Webhook payload validation and error handling

**Technical Details**:
```python
# Webhook routes
POST   /api/v1/webhooks/github

# Repository management
POST   /api/v1/repositories/link
GET    /api/v1/repositories/{id}/setup-webhook
```

**Webhook events to handle**:
- `pull_request` (opened, synchronize, reopened)
- `pull_request_review` (submitted)
- `issue_comment` (created)

**Acceptance Criteria**:
- GitHub webhooks validated and processed reliably
- PR data fetched and stored in PostgreSQL
- Jobs enqueued to Redis within <500ms
- Webhook delivery confirmed to GitHub
- Error handling for malformed payloads
- Idempotency (duplicate webhook prevention)

---

### Task 3: PR Analysis Pipeline - Static Analysis & Categorization (2.5 weeks)

**Objective**: Implement automated code analysis and ML-based PR prioritization

**Deliverables**:
- Code diff fetching and parsing from GitHub
- Language detection and analyzer selection logic
- Pylint/ESLint integration for static analysis
- SonarQube integration for architectural patterns
- PR feature extraction (lines changed, files affected, etc.)
- ML model for priority/complexity classification
- Feature engineering and model training pipeline
- Analysis results storage in MongoDB
- Worker process for async job handling

**Technical Details**:
```python
# Analysis features to extract
- Lines added/deleted
- Files changed count
- File types (web, backend, infra)
- Test coverage impact
- Security-related keywords
- Breaking change indicators
- Author experience level (from git history)
```

**Models**:
- Pre-trained: Use scikit-learn RandomForest for MVP
- Feature set: 15-20 engineered features
- Output: Priority (critical/high/medium/low) + confidence

**Acceptance Criteria**:
- Static analysis runs in <2s per PR
- Priority classification >75% accuracy on test set
- Complexity scoring aligns with developer intuition
- Results stored with analysis metadata
- Error handling for unsupported languages
- Performance metrics logged

---

### Task 4: LLM Integration for Review Summaries (2 weeks)

**Objective**: Integrate OpenAI API for intelligent PR summarization and issue detection

**Deliverables**:
- LangChain setup with GPT-4 integration
- Prompt engineering for review summaries
- Diff-to-text conversion for token efficiency
- Summary caching in Redis and MongoDB
- Suggested reviewers extraction
- Potential issues/concerns identification
- Architectural impact assessment
- Cost tracking and rate limiting
- Fallback strategies for API failures
- Worker process for async LLM jobs

**Technical Details**:
```python
# Prompts to engineer
1. Executive Summary: Key changes in 2-3 sentences
2. Change Analysis: What changed and why
3. Risk Assessment: Potential issues, breaking changes
4. Architectural Impact: Dependencies, patterns affected
5. Reviewer Suggestions: Who should review based on changes

# Token optimization
- Truncate diffs to 4000 tokens max
- Remove comments and formatting
- Summarize large files separately
```

**LLM Parameters**:
- Model: gpt-4-turbo-preview
- Temperature: 0.2 (consistency)
- Max tokens: 2000
- Timeout: 30s
- Retries: 3 with exponential backoff

**Acceptance Criteria**:
- Summaries generated in <5s (cached)
- <2% API error rate
- Cost <$0.10 per PR analyzed
- Summaries are actionable and relevant
- Cache hit rate >60% for repeated reviews
- Graceful degradation on API failures

---

### Task 5: Dashboard Backend & Analytics API (2 weeks)

**Objective**: Create analytics aggregation and dashboard data endpoints

**Deliverables**:
- Analytics event schema in MongoDB
- Review metrics aggregation pipeline
- Bottleneck detection logic
- Dashboard API endpoints for metrics
- Review time trend calculation
- Team velocity tracking
- Quality score aggregation
- Database query optimization
- Caching layer for expensive queries
- Data export functionality (CSV/JSON)

**Technical Details**:
```python
# Dashboard endpoints
GET    /api/v1/teams/{id}/metrics/overview
GET    /api/v1/teams/{id}/metrics/review-time-trend
GET    /api/v1/teams/{id}/metrics/bottlenecks
GET    /api/v1/teams/{id}/metrics/team-velocity
GET    /api/v1/teams/{id}/pull-requests?status=open&sort=-priority
GET    /api/v1/teams/{id}/export?format=csv&start_date=2024-01-01# Metrics to track
- Average review time per PR
- PRs by priority distribution
- Code quality scores over time
- Review cycle time (open to merge)
- Team member review load
- Most reviewed files/components
- Architectural violations by type
```

**Aggregation Pipeline**:
- Real-time: Cache recent metrics in Redis (5-min TTL)
- Batch: Daily aggregation job for historical data
- Storage: PostgreSQL for computed metrics, MongoDB for raw events

**Acceptance Criteria**:
- Dashboard loads in <2s
- Metrics accurate within 5-minute lag
- P95 query latency <500ms
- Supports filtering by date range, team members, repositories
- Data consistency across views
- Export functionality tested with large datasets

---

### Task 6: Frontend Dashboard - React Application (2.5 weeks)

**Objective**: Build responsive React dashboard for PR metrics and review management

**Deliverables**:
- React TypeScript project setup with build pipeline
- Authentication state management with OAuth flow
- Dashboard layout with responsive grid
- PR list view with filtering and sorting
- Metrics visualization (charts, trend lines)
- Real-time status updates (polling/WebSocket)
- Team settings and user management UI
- Rule creation interface
- Notification preferences panel
- Dark mode support
- Mobile-responsive design
- Integration tests for key flows

**Technical Details**:
```typescript
// Component Structure
src/
├── components/
│   ├── Dashboard/
│   │   ├── Overview.tsx         // KPI cards
│   │   ├── ReviewMetrics.tsx    // Charts
│   │   ├── BottleneckAnalysis.tsx
│   │   └── PullRequestList.tsx  // Filterable list
│   ├── Teams/
│   │   ├── TeamSettings.tsx
│   │   └── MemberManagement.tsx
│   ├── Rules/
│   │   ├── RuleList.tsx
│   │   └── RuleEditor.tsx
│   └── Common/
│       ├── Header.tsx
│       ├── Sidebar.tsx
│       └── NotificationCenter.tsx
├── hooks/
│   ├── useAuth.ts
│   ├── useTeam.ts
│   ├── usePullRequests.ts
│   └── useMetrics.ts
├── services/
│   ├── api.ts              // Axios instance with interceptors
│   ├── auth.ts
│   └── analytics.ts
└── pages/
    ├── LoginPage.tsx
    ├── DashboardPage.tsx
    ├── PullRequestPage.tsx
    └── SettingsPage.tsx
```

**Charts to Implement**:
- Review Time Trend (Recharts LineChart)
- Priority Distribution (PieChart)
- Team Velocity (BarChart)
- Code Quality Score (AreaChart)

**Acceptance Criteria**:
- Dashboard accessible and functional in Chrome, Firefox, Safari
- Mobile view works on screens <768px
- Page load time <3s (Lighthouse score >80)
- All API integrations functional
- Form validation on rule/settings inputs
- Error boundaries catch and display errors gracefully
- Unit/integration tests >70% coverage
- Dark mode CSS variables implemented

---

### Task 7: Architectural Rules Engine (1.5 weeks)

**Objective**: Implement pattern-based rule enforcement and violation detection

**Deliverables**:
- Rule schema in PostgreSQL (pattern, naming, dependency, structure types)
- Rule CRUD API endpoints
- Pattern matching logic with regex support
- File naming convention checker
- Dependency analyzer (imports, require statements)
- Directory structure validator
- Rule violation storage and reporting
- Rule severity levels (error, warning, info)
- Bulk rule import from YAML/JSON
- Rule testing/preview before activation

**Technical Details**:
```python
# Rule types and examples
RuleType.PATTERN:
  - "Avoid console.log in production" → regex: r'console\.(log|error|warn)'
  - "Use async/await not Promise.then()" → regex pattern

RuleType.NAMING:
  - React components must start with capital letter
  - Constants must be UPPER_SNAKE_CASE
  - Private methods start with _

RuleType.DEPENDENCY:
  - Forbid 'eval' function
  - Forbid 'moment' (use dayjs)
  - Require test files alongside implementation

RuleType.STRUCTURE:
  - Components must have tests in same directory
  - API routes must be in /routes folder
  - Config files must be in root /config

# API endpoints
POST   /api/v1/teams/{id}/rules
GET    /api/v1/teams/{id}/rules
PUT    /api/v1/teams/{id}/rules/{rule_id}
DELETE /api/v1/teams/{id}/rules/{rule_id}
POST   /api/v1/teams/{id}/rules/{rule_id}/test
POST   /api/v1/teams/{id}/rules/import
```

**Violation Output**:
```json
{
  "rule_id": "uuid",
  "rule_name": "Avoid console.log",
  "severity": "warning",
  "file": "src/components/Button.tsx",
  "line": 42,
  "matches": ["console.log('Button clicked')"],
  "suggestion": "Use logger.debug() instead"
}
```

**Acceptance Criteria**:
- Rules created and saved correctly
- Pattern matching works with common regex patterns
- Violations detected within 2s per rule per PR
- False positive rate <5%
- Rules can be enabled/disabled without deletion
- Bulk import supports YAML format
- Test preview shows matching lines

---

### Task 8: Notification System & Integration (1.5 weeks)

**Objective**: Implement Slack notifications and email alerts for PR events

**Deliverables**:
- Slack API integration with workspace auth
- Slack message formatting for PR notifications
- Email notification service with SendGrid
- Notification preference UI (team and user level)
- Notification queue in Redis
- Throttling/deduplication logic
- Notification delivery retry logic
- Webhook notification endpoint documentation
- Status tracking (sent, delivered, failed)
- Unsubscribe mechanism for emails

**Technical Details**:
```python
# Notification types
class NotificationType(Enum):
    PR_READY_FOR_REVIEW = "pr_ready"
    REVIEW_REQUESTED = "review_requested"
    REVIEW_SUBMITTED = "review_submitted"
    PR_APPROVED = "pr_approved"
    PRIORITY_CRITICAL = "priority_critical"
    ARCHITECTURAL_VIOLATION = "arch_violation"
    REVIEW_BOTTLENECK = "bottleneck"

# Slack message template
{
  "text": "🔍 New PR Review Request",
  "blocks": [
    {
      "type": "section",
      "text": {
        "type": "mrkdwn",
        "text": "*[HIGH] Refactor auth service*\nby @john.doe\n2 files changed, +145 -89"
      }
    },
    {
      "type": "section",
      "fields": [
        {"type": "mrkdwn", "text": "*Priority:*\nHigh"},
        {"type": "mrkdwn", "text": "*Complexity:*\nMedium"}
      ]
    },
    {
      "type": "actions",
      "elements": [
        {"type": "button", "text": {"type": "plain_text", "text": "Review"}, "url": "..."},
        {"type": "button", "text": {"type": "plain_text", "text": "Dismiss"}, "action_id": "dismiss_pr"}
      ]
    }
  ]
}

# API endpoints
POST   /api/v1/teams/{id}/notifications/preferences
GET    /api/v1/teams/{id}/notifications/history
POST   /api/v1/teams/{id}/integrations/slack/connect
POST   /api/v1/teams/{id}/integrations/email/verify
```

**Notification Rules**:
- Critical PRs: Immediate Slack + email
- High priority: Slack within 5 minutes
- Medium/Low: Slack digest 1x/day
- Throttle repeated notifications: Max 1 per PR per hour
- Time-zone aware sending (business hours preference)

**Acceptance Criteria**:
- Slack messages format correctly and include actionable links
- Delivery success rate >99% for Slack
- Email deliverability >95%
- Preferences respected (opt-in/out per notification type)
- Retry logic handles transient failures
- No duplicate notifications sent
- Unsubscribe links in emails functional
- Message timestamps in user's local time zone

---

## Implementation Timeline & Priorities

### Phase 1: MVP Foundation (Weeks 1-4)
- **Task 1**: Core API & Auth (Weeks 1-2)
- **Task 2**: GitHub Integration (Weeks 2-3)
- **Start Task 3**: PR Analysis (Weeks 3-4)

### Phase 2: Intelligence Layer (Weeks 4-8)
- **Complete Task 3**: PR Analysis (Week 4)
- **Task 4**: LLM Integration (Weeks 5-6)
- **Task 5**: Analytics API (Weeks 6-7)
- **Task 7**: Rules Engine (Weeks 7-8)

### Phase 3: User Interface & Polish (Weeks 8-11)
- **Task 6**: Frontend Dashboard (Weeks 8-10)
- **Task 8**: Notifications (Weeks 10-11)
- **Testing & Optimization** (Week 11)

### Phase 4: Launch Preparation (Week 12)
- Security audit (penetration testing)
- Performance optimization and load testing
- Documentation and tutorials
- Beta customer onboarding

**Total Duration**: 12 weeks (3 months) for production-ready MVP

---

## Success Metrics & KPIs

### Performance Metrics
| Metric | Target | Notes |
|--------|--------|-------|
| PR Analysis Latency (P95) | <3s | From webhook to analysis complete |
| Dashboard Page Load | <2s | From browser to interactive |
| API Response Time (P95) | <500ms | For non-analysis endpoints |
| Webhook Processing | >99.9% | Reliability and delivery |
| LLM Summary Generation | 90% success rate | With fallback to basic summary |

### Business Metrics
| Metric | Target | Notes |
|--------|--------|-------|
| Time to First Analysis | <2 weeks | From signup to first PR analyzed |
| User Onboarding Success | >85% | Complete setup without support |
| Feature Adoption | >70% | Of active users using rules engine |
| NPS Score | >40 | Net Promoter Score at launch |
| Monthly Churn | <5% | Free tier monthly churn |

### Quality Metrics
| Metric | Target | Notes |
|--------|--------|-------|
| Code Coverage | >80% | Unit + integration tests |
| Bug Resolution Time | <24h | For critical issues |
| Uptime | 99.9% | Monthly availability |
| Security Incidents | 0 | In first 6 months |

---

## Risk Mitigation

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|-----------|
| GitHub API rate limits | High | Medium | Implement caching layer + batch API calls |
| LLM API cost overruns | High | Medium | Set usage quotas + implement batch processing |
| PostgreSQL scaling issues | High | Low | Use Aurora read replicas + connection pooling |
| Security breach | Critical | Low | SOC2 audit prep + regular pen testing |
| Team turnover during dev | Medium | Medium | Comprehensive documentation + code reviews |
| Market fit validation fails | High | Medium | Launch beta with 5-10 pilot customers early |

---

## Conclusion

CodeMeld's MVP is designed to deliver immediate value through intelligent PR automation while establishing a foundation for enterprise-scale growth. The technology stack balances rapid development (FastAPI, React) with production reliability (Kubernetes, RDS), while the modular architecture enables iterative feature additions based on customer feedback.

Key success factors:
1. **Fast time-to-value**: Analysis results within seconds of PR creation
2. **Minimal friction**: GitHub OAuth flow, automatic webhook setup
3. **Intelligent defaults**: ML-based prioritization without manual configuration
4. **Extensibility**: Custom rules engine for team-specific patterns
5. **Transparency**: Clear analytics on review process bottlenecks

The 12-week development timeline allows for parallel work across frontend, backend, and ML components while maintaining code quality and team velocity. Pilot customers in weeks 8-10 provide early validation before public launch.