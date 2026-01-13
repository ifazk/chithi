# PATH variable

The runner will try to use the same binary that was used to call it for
recursive calls, but if detecting this fails, it will try to call the `chithi`
binary. So it is recommended to have the `chithi` (and `chithi-run`) binaries
via the path variable, otherwise the runner will throw some command not found
errors.
