app [main!] {
    pf: platform "../platform/main.roc",
}

import pf.Stdout
import pf.Arg exposing [Arg]

# How to handle command line arguments in Roc.

# To run this example: check the README.md in this folder

main! : List Arg => Result {} _
main! = |raw_args|

    # get the second argument, the first is the executable's path
    when List.get(raw_args, 1) |> Result.map_err(|_| ZeroArgsGiven) is
        Err(ZeroArgsGiven) ->
            Err(Exit(1, "Error ZeroArgsGiven:\n\tI expected one argument, but I got none.\n\tRun the app like this: `roc main.roc -- input.txt`"))

        Ok(first_arg) ->
            Stdout.line!("received argument: ${Arg.display(first_arg)}")?

            # # OPTIONAL TIP:

            # If you ever need to pass an arg to a Roc package, it will probably expect an argument with the type `[Unix (List U8), Windows (List U16)]`.
            # You can convert an Arg to that with `Arg.to_os_raw`.
            #
            # Roc packages like to be platform agnostic so that everyone can use them, that's why they avoid platform-specific types like `pf.Arg`.
            when Arg.to_os_raw(first_arg) is
                Unix(bytes) ->
                    Stdout.line!("Unix argument, bytes: ${Inspect.to_str(bytes)}")?

                Windows(u16s) ->
                    Stdout.line!("Windows argument, u16s: ${Inspect.to_str(u16s)}")?

            # You can go back with Arg.from_os_raw:
            Stdout.line!("back to Arg: ${Inspect.to_str(Arg.from_os_raw(Arg.to_os_raw(first_arg)))}")