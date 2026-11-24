# The UI project

Windows, templates, resources, styles, and the component data the designer
binds against. This directory is the actor that serves the designer, the
domain model it runs on, the deploy pass, and the tests.

`studio_web_api::project_manager::UiProject` is the other half of the pair: the
serde shape on disk and the ts-rs shape on the wire. Only `model/wire.rs` and
the handler signatures mention it.

| path | lines | holds |
| --- | --- | --- |
| `mod.rs` | 1072 | the actor: one handler per call |
| `component.rs` | 385 | the component refresh chain |
| `model/` | 3860 | the domain model |
| `deploy/` | 926 | project to runtime definition |
| `tests/` | 4235 | 157 tests |

## The shape of an operation

Every operation on `model::Project` takes `&self`, returns a new `Project`, and
records what moved into a `&mut Vec<Change>`. Three things follow:

- **A failed operation is a no-op.** The caller still holds what it started
  from and drops the half-built value. Nothing is rolled back.
- **A `Project` is seven `Arc`s**, so cloning is seven pointer bumps and an
  untouched subtree keeps its pointer.
- **An operation that can fail says so.** With a `ResourceId` in hand,
  `set_resource` cannot fail, so it returns `Self`. "Nothing changed" is
  `Ok(None)`.

Views go back through `Project::with_views_set`, which emits one change per
view. An operation touching the same view twice still produces one
notification for it.

## One member reference, not two `Option<String>`

Four places on the wire hold a pair of optional strings (a display, a text
context item, an action, a template-instance binding). The pair means four
things, one of which means nothing:

| component | member | means |
| --- | --- | --- |
| set | set | that member of that component |
| unset | set | an export of the **enclosing template** |
| unset | unset | not bound yet |
| set | unset | nothing |

Row two is the expensive one: that string is an **export id, not a member
name**, and the pair does not say so. The model holds `Option<MemberRef>`, an
enum of the first two rows, so each is one match arm and row four cannot be
built. `wire::member_ref_from_wire` rejects row four with
`ComponentWithoutMember`; only a hand-edited file can hold one.

## The actor

`data` and `model` are two views of one project, replaced together and never
apart. A stored project the model refuses fails at `create`, not on whichever
call touches it first; one written underneath us that the model refuses is not
taken at all, and the pair in memory stays the one known to hold together.

A handler is the boundary plus one model call:

```rust
let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
let id = model::TemplateInstanceId::new(&call_data.id)?;
let bindings = model::wire::bindings(call_data.bindings)?;

self.apply_model_update(|project, changes| {
    project.set_template_instance_bindings(changes, &view, id, bindings).map(Some)
}).await
```

Those first lines are the whole boundary: an id the client made up is rejected
there rather than four calls deep. `apply_model_update` stores what the
operation produced and announces what it recorded.

The outbound conversion runs per call and is not worth avoiding:
`fs_collection` serialises the whole project and rewrites its file every time,
so a `UiProject` has to exist anyway.

## The wire layer

`model/wire.rs` is 943 lines and that is the price of the split. Each
conversion is a `From` or `TryFrom`, which the orphan rule allows because
`impl From<&LocalType> for ForeignType` puts a local type first (RFC 2451).

The reverse does not work, and the shape looks identical, so: `impl<K, V, W>
From<&Collection<K, V>> for Arc<HashMap<String, Arc<W>>>` is rejected, because
`Self` is built from foreign constructors and carries the uncovered parameter
`W` ahead of the first local type. Per concrete pair it would be legal, but
that is seven identical impls. It is `Collection::to_wire` instead.

Three free functions remain: `member_ref_from_wire` and `view_key` take more
than one input, `bindings` converts one foreign map into another.

## The deploy pass

`deploy/` produces `ui_model::Definition`. It sits beside the model, not in it,
for two reasons. An operation fails at the first problem with one
`UiModelError`; a deploy reports every problem with the path it was found at,
which is `Validation` plus the visit-everything rule in `deploy_items`. And it
produces `ui_model`, a second foreign format the model does not need to know.

The work is flattening: a template's controls deploy once per instance, with
ids prefixed by the chain of instance ids and positions made absolute. A
reference to an enclosing export resolves through the binding its instance
supplied, so what reaches the runtime is always a component and a member.

A template is also deployed alone, so an unused one still reports its errors;
those controls are discarded.

## What the tests guard

`tests/` calls the internal API directly. Each operation is checked on what it
returns, what it emitted, and what the project then holds, including the parts
it was not aimed at.

These were confirmed by breaking the code and watching the test fail:

- Two usages in one view both clear and emit one change. Reading the view from
  `self` instead of the accumulator loses one.
- Clearing a template export reaches the instances nested in it.
- The wire conversion is a fixed point. Since that only catches drift *between*
  the two passes, outbound values are also asserted directly.
- An owner-export reference survives the wire, and resolves through its
  instance binding at deploy.
- A deploy reports every broken control in a view, not the first. Only the
  two-controls-in-one-view test pins this; the other multi-error test provokes
  its errors in separate branches.

One hazard is reduced, not removed: the read phase of `clear_component_usage`
still consults `self` for views it has not touched.

## Known behaviour: the scope of `clear_export`

`clear_export` drops the export and the references to it **inside the owning
template**. Instances of that template in *other* views keep a binding slot
keyed by an export that is gone. Deploy rejects it ("Binding sur export '...'
inexistant"), so it surfaces, but only then. Both halves have a test.

That is deliberate, on the grounds that surfacing it at deploy may be the
intent. If it is not, the fix is a project-level `on_clear_template_export`
next to `on_change_template`.
