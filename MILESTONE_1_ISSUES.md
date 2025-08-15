# Milestone 1 Issues - Core Value & Speed (Weeks 0-6)

## Epic: Provider Router System
**Goal**: Replace direct model calls with a pluggable provider system

### Issue #1: Implement Provider Router Architecture
**Priority**: P0 (Blocker)
**Estimate**: 3 days
**Labels**: `enhancement`, `architecture`, `milestone-1`

**Description**:
Create a provider router system that abstracts AI model access behind a common interface, enabling local (Ollama) and fallback (heuristic) providers.

**Acceptance Criteria**:
- [ ] Create `backend/src/ai_engine/providers/` module structure
- [ ] Define `Provider` trait with `complete()`, `analyze()`, `health()` methods
- [ ] Implement `OllamaProvider` with HTTP client to localhost:11434
- [ ] Implement `HeuristicProvider` as intelligent fallback
- [ ] Create `ProviderRouter` that selects provider based on config/policy
- [ ] Wire into existing `AIEngine` replacing direct model calls
- [ ] Unit tests with mocked HTTP for Ollama
- [ ] Integration tests covering fallback scenarios
- [ ] Metrics recording per provider (success/latency)

**Implementation Notes**:
- Use `reqwest` for HTTP client with timeout handling
- Config-driven provider selection (prefer local, fallback to heuristic)
- Preserve existing API compatibility
- Add provider health checks to `/health` endpoint

---

## Epic: Context Management System
**Goal**: Intelligent repo-aware context selection for better suggestions

### Issue #2: Implement Repository Scanner
**Priority**: P0 (Blocker)
**Estimate**: 4 days
**Labels**: `enhancement`, `context`, `milestone-1`

**Description**:
Build a repository scanner that indexes files, extracts ASTs, and prepares context for AI providers.

**Acceptance Criteria**:
- [ ] Create `backend/src/context/` module structure
- [ ] Implement `repo_scan.rs` using `walkdir` + `ignore` crate
- [ ] Filter files by language and size limits
- [ ] Extract basic file metadata (language, LOC, last modified)
- [ ] Implement `ast_graph.rs` using existing tree-sitter parsers
- [ ] Extract symbols (functions, classes, imports) from ASTs
- [ ] Store scan results in memory with refresh capability
- [ ] Unit tests with fixture repositories
- [ ] Performance tests (scan time for repos of various sizes)

**Implementation Notes**:
- Respect .gitignore and .uaidaignore files
- Support Python, JavaScript, TypeScript, Rust, Go initially
- Incremental scanning for file changes
- Memory-efficient storage of AST data

### Issue #3: Implement Context Selection Engine
**Priority**: P0 (Blocker)
**Estimate**: 6 days
**Labels**: `enhancement`, `context`, `milestone-1`

**Description**:
Build intelligent context selection using embeddings and graph analysis to provide relevant code context.

**Acceptance Criteria**:
- [ ] Implement `embeddings.rs` with local embedding model
- [ ] Create sqlite-vss or FAISS index for semantic search
- [ ] Implement `selection.rs` with MMR (Maximal Marginal Relevance) algorithm
- [ ] Consider file recency, centrality, and relevance scores
- [ ] Return `ContextPackage` with files, spans, symbols, related tests
- [ ] Configurable context window size limits
- [ ] Unit tests with known relevant/irrelevant examples
- [ ] Benchmark context quality with acceptance rate metrics

**Implementation Notes**:
- Use sentence-transformers or similar for embeddings
- Balance relevance vs diversity in selection
- Cache embeddings to avoid recomputation
- Expose context selection via internal API

---

## Epic: Sandbox Execution System
**Goal**: Safe, isolated code execution for testing and validation

### Issue #4: Implement Docker-based Sandbox Runner
**Priority**: P0 (Blocker)
**Estimate**: 6 days
**Labels**: `enhancement`, `sandbox`, `milestone-1`

**Description**:
Create a sandbox system for safely executing code changes and running tests in isolated environments.

**Acceptance Criteria**:
- [ ] Create `backend/src/sandbox/` module structure
- [ ] Implement `python.rs` runner with pytest support
- [ ] Implement `node.rs` runner with jest support
- [ ] Docker container management with resource limits
- [ ] Capture stdout/stderr, exit codes, and execution time
- [ ] Optional test coverage collection
- [ ] Timeout enforcement and cleanup
- [ ] Unit tests with mock Docker daemon or lightweight images
- [ ] Integration tests with real containers

**Implementation Notes**:
- Use `bollard` crate for Docker API interaction
- Predefined language-specific Docker images
- Network isolation and filesystem restrictions
- Configurable resource limits (CPU, memory, time)

---

## Epic: Agent Loop System
**Goal**: Orchestrated multi-agent workflow for autonomous development

### Issue #5: Implement Core Agent Framework
**Priority**: P0 (Blocker)
**Estimate**: 8 days
**Labels**: `enhancement`, `agents`, `milestone-1`

**Description**:
Build the foundational agent system with planning, retrieval, code generation, and testing capabilities.

**Acceptance Criteria**:
- [ ] Create `backend/src/agents/` module structure
- [ ] Implement `planner.rs` - converts goals to actionable steps
- [ ] Implement `retriever.rs` - queries ContextManager for relevant code
- [ ] Implement `codegen.rs` - generates patches using Provider system
- [ ] Implement `testgen.rs` - creates tests for Python/Node
- [ ] Implement basic `reviewer.rs` and `risk.rs` (stubs for M1)
- [ ] Agent orchestration with timeouts and budget enforcement
- [ ] State persistence for long-running operations
- [ ] Unit tests for each agent type
- [ ] Integration tests for full agent loop

**Implementation Notes**:
- Use async/await for concurrent agent execution
- Implement budget tracking (time, LOC, file count)
- Error handling and graceful degradation
- Structured logging for agent decisions

---

## Epic: REST API & Documentation
**Goal**: Well-documented, validated API endpoints

### Issue #6: Implement Core API Endpoints
**Priority**: P0 (Blocker)
**Estimate**: 5 days
**Labels**: `enhancement`, `api`, `milestone-1`

**Description**:
Create the REST API endpoints for the agent loop system with proper validation and documentation.

**Acceptance Criteria**:
- [ ] Implement `POST /api/v1/plan` endpoint
- [ ] Implement `POST /api/v1/patch` endpoint  
- [ ] Implement `POST /api/v1/run-tests` endpoint
- [ ] Implement `GET /api/v1/risk-report/{id}` endpoint
- [ ] Implement `GET /api/v1/artifacts/{id}` endpoint
- [ ] Implement `POST /api/v1/rollback` endpoint
- [ ] Add OpenAPI schema generation with `utoipa`
- [ ] Add Swagger UI at `/docs` endpoint
- [ ] JSON schema validation for all inputs
- [ ] Structured error responses with error codes
- [ ] Integration tests for all endpoints
- [ ] Postman collection for manual testing

**Implementation Notes**:
- Use `validator` crate for input validation
- Consistent error response format
- Rate limiting preparation (headers, but not enforcement yet)
- CORS configuration for development

---

## Epic: Observability Foundation
**Goal**: Metrics, tracing, and monitoring infrastructure

### Issue #7: Implement Basic Observability
**Priority**: P1 (Important)
**Estimate**: 5 days
**Labels**: `enhancement`, `observability`, `milestone-1`

**Description**:
Add foundational observability with metrics and tracing for monitoring system health and performance.

**Acceptance Criteria**:
- [ ] Add Prometheus metrics middleware
- [ ] Implement basic counters: requests_total, provider_calls_total
- [ ] Implement histograms: request_duration, provider_latency
- [ ] Add OpenTelemetry tracing spans for agent steps
- [ ] Expose metrics at `/metrics` endpoint
- [ ] Add correlation IDs to all logs
- [ ] Update health endpoint with detailed system status
- [ ] Basic Grafana dashboard templates
- [ ] Documentation for metrics and their meanings

**Implementation Notes**:
- Use `prometheus` and `opentelemetry` crates
- Structured logging with `tracing`
- Prepare for future alerting rules
- Keep overhead minimal

---

## Epic: VS Code Integration
**Goal**: Developer-friendly IDE experience

### Issue #8: Create VS Code Extension MVP
**Priority**: P1 (Important)
**Estimate**: 5 days
**Labels**: `enhancement`, `ide`, `milestone-1`

**Description**:
Build a minimal VS Code extension that demonstrates the core capabilities with ghost text and patch proposals.

**Acceptance Criteria**:
- [ ] Create `extensions/vscode/` directory structure
- [ ] Implement ghost text completion using `/api/v1/complete`
- [ ] Add "Propose Patch" command calling `/api/v1/plan` + `/api/v1/patch`
- [ ] Show patch diff in VS Code diff viewer
- [ ] Add "Run Tests" command calling `/api/v1/run-tests`
- [ ] Display results in output panel
- [ ] Configuration for backend URL
- [ ] Basic error handling and user feedback
- [ ] Package as .vsix for local installation

**Implementation Notes**:
- Use TypeScript and VS Code Extension API
- HTTP client for backend communication
- WebSocket support for future real-time updates
- Follow VS Code extension best practices

---

## Epic: Evaluation Pipeline
**Goal**: Reproducible benchmark infrastructure

### Issue #9: Bootstrap Evaluation Infrastructure
**Priority**: P1 (Important)
**Estimate**: 4 days
**Labels**: `enhancement`, `evaluation`, `milestone-1`

**Description**:
Create the foundation for running reproducible evaluations on standard benchmarks.

**Acceptance Criteria**:
- [ ] Create `scripts/evals/` directory structure
- [ ] Implement HumanEval+ dataset download and runner
- [ ] Implement small SWE-bench Lite subset runner
- [ ] Create Docker images for isolated evaluation runs
- [ ] Add `make eval` command with configurable parameters
- [ ] JSON result format with artifacts (diffs, logs)
- [ ] Basic HTML report generation
- [ ] Store results in `docs/evals/` with timestamps
- [ ] CI job for nightly evaluation runs

**Implementation Notes**:
- Use existing evaluation frameworks where possible
- Containerized execution for reproducibility
- Version control for datasets and results
- Prepare for expanded benchmark suites

---

## Database & Persistence

### Issue #10: Implement Database Layer
**Priority**: P1 (Important)
**Estimate**: 3 days
**Labels**: `enhancement`, `database`, `milestone-1`

**Description**:
Set up PostgreSQL database with migrations and basic data models.

**Acceptance Criteria**:
- [ ] Create database schema with sqlx migrations
- [ ] Implement core tables: users, projects, runs, artifacts, completion_logs
- [ ] Add database connection pool to main.rs
- [ ] Update health endpoint to include database status
- [ ] Basic CRUD operations for core entities
- [ ] Database seeding for development
- [ ] Backup and restore documentation

**Implementation Notes**:
- Use sqlx with compile-time checked queries
- Environment-based configuration
- Connection pooling and retry logic
- Prepare for horizontal scaling

---

## Milestone 1 Success Criteria

**Demo Scenario**: End-to-end autonomous PR generation
1. User provides a goal: "Add input validation to the login function"
2. System scans repo, finds relevant files
3. Agent loop: Plan → Retrieve context → Generate patch → Create tests → Validate
4. Returns: PR diff, test results, risk assessment, rollback command
5. VS Code extension shows the patch with one-click apply

**Metrics to Track**:
- API response times (p50, p95, p99)
- Provider success rates and fallback frequency  
- Context selection relevance (measured by suggestion acceptance)
- Evaluation scores on HumanEval+ and SWE-bench subset
- System uptime and error rates

**Deliverables**:
- Working REST API with OpenAPI documentation
- VS Code extension demonstrating core features
- Evaluation results published with reproducible pipeline
- Docker deployment ready for testing
- Comprehensive documentation for setup and usage

---

## Getting Started for Contributors

1. **Setup Development Environment**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/universal-ai-dev-assistant
   cd universal-ai-dev-assistant
   make install  # Install dependencies
   make dev      # Start development server
   ```

2. **Run Tests**:
   ```bash
   make test     # Unit and integration tests
   make eval     # Run evaluation suite
   ```

3. **Pick an Issue**:
   - Start with issues labeled `good-first-issue`
   - Check acceptance criteria before beginning
   - Ask questions in issue comments

4. **Submit PR**:
   - Include tests for new functionality
   - Update documentation as needed
   - Ensure CI passes before requesting review

---

**Estimated Timeline**: 6 weeks for Milestone 1 completion
**Next Milestone**: PR Quality & Safety (Weeks 7-12)