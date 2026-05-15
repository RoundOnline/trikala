# AI prompt cards

Pair-programming with a model is part of trikala's learning curve.
Throughout the docs and templates you will see blocks like this:

```text
> 🤖 ask Claude: "Why does my game freeze for a second every time
>                the camera rotates?"
> expected: a one-paragraph diagnosis + a one-line code patch
```

Per axiom C6 every prompt card uses this exact shape. The point isn't
that you must use Claude (or any one model); it's that the *format*
is uniform so a reader scanning a tutorial can spot the assist points.

## When to drop a prompt card

- **Right before a step that requires domain knowledge** the reader
  may not have (linear algebra for a camera, audio DSP for a filter).
- **Right after introducing a new file** that is mostly boilerplate.
- **Not** for trivial code that's better just written out.

A prompt card is a shortcut for the reader, not a crutch for the
author. If half a tutorial is prompt cards, write less tutorial.

## What a good `expected:` line looks like

It tells the reader what they're aiming for so they can judge a
bad model output and re-prompt:

- "Two-paragraph explanation, no code"
- "A single 20-line WGSL fragment"
- "A list of three approaches, ranked by complexity"

## What a bad `expected:` line looks like

- "The answer" (too vague)
- "Production-ready code" (no such thing in one prompt)
- (omitted — reader can't tell if the model failed)
