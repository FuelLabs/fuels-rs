# Pre-uploading code

If you have a script or predicate that is larger than normal or which you plan
on calling often, you can pre-upload its code as a blob to the network and run a
loader script/predicate instead. The loader can be configured with the
script/predicate configurables, so you can change how the script/predicate is
configured on each run without having large transactions due to the code
duplication.

## Scripts

A high level pre-upload:

```rust,ignore
{{#include ../../e2e/tests/scripts.rs:preload_high_level}}
```

The upload of the blob is handled inside of the `convert_into_loader` method. If you
want more fine-grained control over it, you can create the script transaction
manually:

```rust,ignore
{{#include ../../e2e/tests/scripts.rs:preload_low_level}}
```

## Predicates

You can prepare a predicate for pre-uploading without doing network requests:

```rust,ignore
{{#include ../../e2e/tests/predicates.rs:preparing_the_predicate}}
```

Once you want to execute the predicate, you must beforehand upload the blob
containing its code:

```rust,ignore
{{#include ../../e2e/tests/predicates.rs:uploading_the_blob}}
```

By pre-uploading the predicate code, you allow for cheaper calls to the predicate
from subsequent callers.
