# The core project

Components, the bindings between them, the plugins they instantiate and the
templates that group them. This directory is the actor that serves the
designer, the domain model it runs on, and the tests.

| path | lines | holds |
| --- | --- | --- |
| `mod.rs` | 723 | the actor: one handler per call |
| `model/` | 2851 | the domain model |
| `tests/` | 2314 | 88 tests |

Inside `model/`: `project.rs` (1213) holds every operation, `wire.rs` (487)
the conversions to and from the API types, `types.rs` (398) the data types,
`naming.rs` (201) the deploy-time id simulation, and `ids.rs`, `view.rs`,
`error.rs` and `change.rs` the rest.

The shape of an operation, how the actor keeps `data` and `model` in step, and
the module visibility rule are the same as the UI project's;
`ui_project/DESIGN-NOTES.md` describes all three. What the two models share is
`Collection`, the `Id` and `IdError` traits it is written against, and the
`define_id!` macro, in `project_manager/model/`. Ids, id kinds and errors are
each project's own.

## Three things that are different in kind from the UI side

### 1. A binding has no id

Its id *is* its four parts joined: `source:state:target:action`.
`Binding::id()` derives it, so the key a binding is stored under and the data
inside it cannot disagree. Renaming a component is one operation that re-keys
every binding mentioning it, emitting the old key's removal, the rename and
the new key in that order, so the designer never holds a dangling one.

A derived id contains ':', so `BindingId` validates as non-empty rather than
as an authored id, exactly like `PluginId`. Giving them their own types is
what makes that readable.

### 2. A component can instantiate a template

A plugin answers "what type is member X" directly. A template answers it
through its exports, which point at one of *its* components, whose definition
may be another template. So `definition_member`, `definition_config_default`
and `definition_check_config_value` recurse, with a depth guard, and live on
`Project` rather than on the types, since only the project can resolve a
`TemplateId`.

Three checks fall out, all enforced before anything is written: a template
cannot contain itself directly or through another; a config item cannot be
exported twice, since both exports would try to supply it; and a component an
export points at cannot be deleted or silently renamed.

Exporting a config item moves it: the inner component stops carrying the
value, and every component instantiating the template gains it at its default.

### 3. Ids are generated at deploy time, and can collide

A template's components are instantiated once per component using the
template, and each level substitutes `{{id}}`. Two chains can land on the same
final id, which makes the project undeployable. `naming.rs` simulates that
before an operation is allowed, with a cycle guard so malformed stored data
cannot loop forever, and a sorted report so the same collision reads the same
way twice.

One consequence is easy to meet by accident: **a template containing a
component with a fixed id can only be used once**, because instantiating it
twice generates that id twice. That is why copying components into a template
names them `{{id}}-<source id>`.

The dry run runs after the plain checks, not before: a component id that is
simply taken reports as such rather than as a deploy-time collision.

## Two shapes specific to this model

**`ViewKey::Project` instead of a null `templateId`.** Every core call carries
`templateId: string | null`. `wire::view_key` turns that into a `ViewKey`
once, at the boundary, and nothing inside the model tests for null again.

A `View` is then a key and the components and bindings it names, and an
operation is written once against it for both the project and a template. The
key travels with the contents because `views()` hands them out in a batch,
where there is no parameter to say which is which. Exports are not part of it:
a template is a view plus exports, and `with_view_set` reads them back and
leaves them alone, so an operation that changes them says so in a step of its
own. Only two do.

**Instances are derived, not stored.** They are the distinct `instance_name`s
of the plugins, so `Project::instances()` computes them. There is no instance
list that can fall out of step with the plugin list.

Unlike the UI project, putting a view back emits nothing: the core designer is
told about individual components and bindings, so each operation emits as it
goes.

## What the tests guard

`tests/` calls the internal API directly: model operations and the wire
conversions, as functions. Nothing spawns an actor or touches the filesystem.
Each operation is checked on what it returns, what it emitted, and what the
project then holds, including the parts it was not aimed at.

These were confirmed by breaking the code and watching the test fail:

- Renaming a component re-keys its bindings. Keeping the old key fails it.
- A fixed id inside a template stops a second use. Dropping the dry run from
  `set_component` fails it.
- A selection holding one undeletable component removes none of them, and
  announces none of them. Checking each component as it is removed rather than
  all of them first fails it, and only the "announces none" half catches it:
  the project is discarded on failure either way.
- Exporting a config item takes it off the inner component. Writing the
  template back from the snapshot taken before that update fails it, which is
  how the bug was found in the first place.

## Beyond the model

`resolver.rs` flattens a project: `{{id}}` filled in, exported config supplied
from outside, member exports walked through to the component behind them.
Everything downstream starts there, and nothing downstream knows templates
exist.

`validation.rs` checks the result against what is running. `deploy.rs` plans
what to change on the instances. `import/` works out what taking plugins and
components in from elsewhere would do.

All three are functions of values: what is online arrives as a snapshot the
actor gathered, never as a lookup they make themselves. That is what lets them
be tested without a registry, and it is why each has a prepare step whose
answer the designer is shown before anything is written.

Import is three steps rather than a diff, because removing a plugin takes its
components, which takes their bindings and the template exports naming them.
That cascade is computed once, in `import/update.rs`, before the user is
asked; `import/apply.rs` then writes the picked subset in an order where each
write's ground has been prepared. `clear-template-export` is the same
machinery with a single change in it.

Two things are worth knowing about the diff. A plugin counts as changed only
when its version moved: members that shift under the same version are not
reported. And a component that would collide with an id a template generates
is left out of the import rather than refusing the whole of it.
