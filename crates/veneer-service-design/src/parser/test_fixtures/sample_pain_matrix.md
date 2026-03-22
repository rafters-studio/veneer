## 1. Scoring Rubric

| Dimension | Weight | Scale Max |
| --- | --- | --- |
| Frequency | 0.3 | 5 |
| Severity | 0.3 | 5 |
| Breadth | 0.2 | 5 |
| Workaround Cost | 0.2 | 5 |

## 2. Theme Table

| Theme | Frequency | Severity | Breadth | Workaround Cost | Composite | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Config complexity | 4 | 3 | 5 | 3 | 3.8 | 4 of 5 users mentioned this |
| Token sync drift | 5 | 4 | 3 | 4 | 4.1 | Observed in 3 codebases |

## 3. Ranked Priorities

| Priority | Theme | Composite |
| --- | --- | --- |
| 1 | Token sync drift | 4.1 |
| 2 | Config complexity | 3.8 |

## 5. Disconfirmation Log

| Assumption | Evidence Against | Status |
| --- | --- | --- |
| All devs hate GUIs | 2 of 5 prefer visual tools | Partially disconfirmed |

## 6. Probe / Experiment Backlog

| Question | Method | Success Metric |
| --- | --- | --- |
| Does auto-sync reduce drift? | A/B test with 10 teams | Drift incidents drop 50% |
| Is CLI sufficient for designers? | User interviews | 80% task completion rate |
