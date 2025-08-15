# Pull Request Template

## Description
Briefly describe the changes and their purpose.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Refactoring (no functional changes)

## Day-0 Requirements Checklist (MANDATORY)
- [ ] **Tests**: Unit tests added/updated and all tests pass
- [ ] **Metrics**: New metrics added to /metrics endpoint (if applicable)
- [ ] **Docs**: Documentation updated (README, /docs endpoint, etc.)
- [ ] **Risk Assessment**: Security and performance impact evaluated

## Technical Details
- [ ] Code follows the project's coding standards
- [ ] Self-review of code completed
- [ ] Code is well-commented, particularly in hard-to-understand areas
- [ ] Changes generate no new warnings or errors

## Testing
- [ ] Unit tests pass (`make test`)
- [ ] Integration tests pass (if applicable)
- [ ] Manual testing completed
- [ ] Metrics are visible in `/metrics` endpoint
- [ ] Swagger docs updated at `/docs`

## Dependencies
- [ ] No new dependencies added OR new dependencies are justified
- [ ] Cargo.lock updated (if dependencies changed)
- [ ] Security scan passed (`make lint`)

## Deployment
- [ ] No database migrations required OR migrations are backward compatible
- [ ] No breaking API changes OR changes are documented
- [ ] Environment variables documented in .env.example

## Observability
- [ ] Appropriate logs added
- [ ] Metrics collection points identified
- [ ] Error handling implemented
- [ ] Tracing spans added (if applicable)

## Security
- [ ] Input validation implemented
- [ ] No secrets hardcoded
- [ ] Rate limiting considered
- [ ] Authentication/authorization checked

## Performance
- [ ] No performance regressions
- [ ] Resource usage within acceptable limits
- [ ] Caching strategy considered

## Related Issues
Closes # (issue number)

## Screenshots/Logs
If applicable, add screenshots or log outputs.

## Additional Notes
Add any other context about the PR here.