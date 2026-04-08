# AI Venture Generator

1. Idea Agent → creates issue

2. Human → approves (/approve)

3. Design Phase
   3.1 Agent → creates feature branch
   3.2 Agent → opens PR with design doc
   3.3 Human → reviews / edits PR
   3.4 Human → merges PR

4. Planning Phase
   4.1 Agent → parses design doc
   4.2 Agent → creates task issues

5. Execution Phase
   5.1 Human → assigns task to agent
   5.2 Agent → implements → opens PR
   5.3 Human → reviews PR
   5.4 Human → triggers iteration (/iterate)
   5.5 Agent → updates PR
   5.6 Human → merges PR
   5.7 Agent → closes issue

6. (Later) Automation hooks
