import { Before } from "@cucumber/cucumber";

// Every scenario traces to an acceptance criterion; an untagged scenario fails the run.
Before(function ({ pickle }) {
  if (!pickle.tags.some((tag) => tag.name.startsWith("@criterion-"))) {
    throw new Error(`Scenario "${pickle.name}" has no @criterion- tag`);
  }
});
