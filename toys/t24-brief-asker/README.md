# T24: brief asker

A draft agent (`brief-asker.md`) that asks the questions legion's brief-writer would otherwise leave open, then hands the answers to the brief-writer to fulfil. Written with Sean on 2026-11-15, from the idea that an agent asked to build blind always fails. Not wired into any plugin.

## How it fits the brief-writer

The brief-writer lands a `brief` document (schema `01a138b1-7e24-77a3-96a9-6094096d8f39`). Every statement in it must cite a prototype, a review, a reference one of those introduced, or an intent line; it never fills a gap. A review is a named person's words, verbatim. So the asker hands over the requester's words as raw material (who, date, the prototypes asked about, each question and answer verbatim), and the brief-writer builds its own review entry from them, with its own ids.

## What has to change for it to work

1. **The brief-writer's dispatch.** Its contract is "exactly an intent id and the prototype canvas links, nothing else". It needs to accept one more line, `Review: <where>`, or it never reads the answers. That is a change to legion's agent, for legion and Sean.
2. **Briefs need prototypes.** The `brief` schema requires at least one prototype, so the asker stops when there is none. An issue with no prototype (a patch, an ad-hoc request) has no brief to fulfil. If those need briefs too, the asker would fill the first brief's `open[].answer` fields (doc type `design-brief`, which holds the operator's words verbatim) instead, or the `brief` schema would allow no prototype.
3. **Where it lives.** The rafters plugin (Sean, 2026-11-15).
4. **Provenance (legion, 01a1de9b and after).** The brief-writer puts a review it knows only second-hand under `open`. An agent recording the requester's words reads as second-hand, so a correct brief-writer could move every answer to open. Either the rule names an asker's verbatim, named and dated record as first-hand, or the requester posts the answers themselves. Sean's call.
