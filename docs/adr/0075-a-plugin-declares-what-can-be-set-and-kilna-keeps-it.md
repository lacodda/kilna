# 0075 — A plugin declares what can be set, and kilna keeps it

Date: 2026-11-05
Status: Accepted

## Context

A plugin that wanted settings had to bring its own way of holding them.
kilna-plugin-cut had a JSON file beside its executable, and a person who wanted to
rename a channel opened it in a text editor. Nothing in the window said which
plugins were there except their buttons, a plugin that could not run said why in
the head of every work, and a command with `target: release` was drawn nowhere.

## Decision

**A manifest may declare `settings`.** Each has a key, a label, a type - text,
number, boolean or choice - and a default. kilna draws the form in Settings, holds
a value to what the setting declares, and hands every run the values under
`settings`. Protocol stays 1: the field is optional on both sides, and an old plugin
ignores what it is given.

**kilna keeps only what was changed**, in `plugin_settings`, by the executable's
name, past the operations log. A setting left alone keeps following the manifest's
default, so a default changed in a plugin's next release reaches everyone who left
it alone, and a value can be taken back. A row outlives the plugin, so a reinstall
finds its settings where they were. A value that no longer fits what the setting
declares is kept and not read: the default stands.

**The default is what the plugin does now**, and it may compute it each time it is
asked. kilna-plugin-cut offers its file's values as its defaults, so the form shows
the truth before anything was set, and the file goes on working for a person who
prefers it. The layers are one direction: defaults, the file, then the window.

**A plugin can be turned off**, without uninstalling it: it offers nothing, cannot
be run, and stays listed so it can be turned back on. A plugin that cannot run says
that it cannot in the head of a work, with a link; the reason is read on the screen
that has room for it.

**A new target, `score`.** A command on a work's Score tab is handed the kind's axes
and may return `score`: marks and a reason. It goes through the call an answer of the
assistant's goes through, so it is held to the axes the same way, and it lands as a
proposal in a chat named after the plugin. Nothing is written until the person keeps
it, and then it is an ordinary score naming the plugin as its judge.

**Scores that wait stand on the Score tab**, whoever proposed them. Until now one
waited in a chat nobody had a reason to open, and a plugin's would have been the
first to arrive from the tab itself. It is held back while judging blind: a proposal
is somebody's verdict.

**Words by language**, optionally, beside every word a manifest shows. Settings made
a Russian window an English form otherwise, and the plugin is the only one who knows
its own words.

**Asking a plugin what it is has a limit of five seconds.** The list is read whenever
a screen with a bar opens, and a plugin that never answered held the whole list, on
the main thread until this version.

## Why not the alternatives

**kilna editing the plugin's own file.** Every plugin would need the same file shape,
or kilna would need to know each one. A declared setting is the contract, and the
file stays the plugin's business.

**A generic JSON box per plugin.** It is what a text editor already gives, with less
to say about what a value may be.

**Secrets among the settings.** They are kept in the workspace, go into every backup
and travel in the clear. A plugin with a token asks the system's keychain, as ADR 0004
expected. There is deliberately no `secret` type to suggest otherwise.

## Consequences

A manifest at odds with itself - a key named twice, a choice with nothing to choose,
a default its setting does not take - leaves the plugin listed with what is wrong and
unable to run, rather than a form that cannot be filled in.

A value is checked against the manifest as it stands, so setting one starts the
plugin to ask. That is milliseconds, off the main thread.

`backup::restore`, written for a restore flow that was never built, is gone with its
promises in the docs: a restore is steps a person takes with kilna closed, and the
tests follow those steps.
