# driven's file formats

driven understands multiple distinct file formats.

The primary format driven understands is the ".driven" file. This file uses a custom format documented below.

It also understands the .envrc format that `direnv` uses. This includes parsing `.envrc` files as arbitrary bash code with a few helper functions.
This format is only supported for backwards compatibility reasons, and it's encouraged that you do not use it.

## The `.driven` format

First, let's look at a few example `.driven` files:

```driven
POSTGRES_USER = "admin"
POSTGRES_PASSWORD = "password"

DATABASE_URL = "postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}/database"
```

Note that while the above uses something that looks similar to bash environment
variable substitution, this is *not* being parsed by a shell. Constructs like
`"${#VAR}` to substitute in the length of the string will not work.

```driven
allow shell-exec

POSTGRES_PASSWORD = "$(pass show databases/dev-postgres | head -n 1)"
```

This example shows actually running a shell command to get the value of an
environment variable. Doing so is discouraged, and in fact to run subshells it
is required that the `allow shell-exec` command be put at the beginning of the
file.

This allows driven to warn when a given `.driven` file will do a dangerous
operation. Requiring that the "allow" directive is the first line in the file
also makes it easier for users to quickly see this when reading a `.driven`
file.

```driven

internal not_exported = "123"

ONE_TO_SIX = "${not_exported}456"
```

It's possible to mark variables as being useful for driven, but not exported,
by using the `internal` keyword. In the `DATABASE_URL` example above, it might
make sense to have the `USER` and `PASSWORD` values as `internal` if the
`DATABASE_URL` variable is the only one actually being used.

Even though we haven't yet quoted the variable name, it is possible to.

```driven
"who puts spaces in env vars" = "really, no one does this"

internal prefix="PROJECT"

"${prefix}_USER" = "username"
"${prefix}_PASS" = "password"
```

The above shows the two cases where quoting variable names is required: if they
contain spaces (or other unusual characters), or if they wish to use the `${}`
or `$()` constructs for any reason.
