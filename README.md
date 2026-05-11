<p align="center">
    <img width="300px" src="https://gitlab.com/mech-lang/assets/-/raw/v0.3-beta/images/mika/mika-pose-hello.png?ref_type=heads">
</p>
<p align="center">
  <img width="500px" src="https://mech-lang.org/img/logo.png" alt="Mech Logo">
</p>

**Mech is for building** data-driven, reactive systems like **robots**, games, embedded devices and more.

**It simplifies data** distribution, transformation, and analysis **so you can focus** on your project.

- [Try Mech](https://try.mech-lang.org) online in your browser. 
- [Read the docs](https://docs.mech-lang.org) to learn the language.
- [Follow our blog](https://mech-lang.org/blog/) to stay updated on new developments.

***

The following code implements FizzBuzz in Mech:

<p align="center">
  <img width="600px" src="https://gitlab.com/mech-lang/assets/-/raw/ea310ad7a32025d34e6a60dc155a520266bd42b0/images/code/fizzbuzz.png" alt='The classic FizzBuzz program expressed in Mech.'>
</p>

See the docs for an [extended version](https://docs.mech-lang.org/examples/fizzbuzz.html) that runs live in your browser.

Some notable features of Mech are demonstrated in this short program:

- **Concise Syntax**: Expressive and flexible, with no need for keywords or semicolons; Mech programs are generally shorter than in other languages.
- **Broadcast Operations**: Vector operations apply elementwise automatically, removing explicit loops and enabling efficient vectorization.
- **Logical Indexing**: Vector elements are conditionally selected using broadcast logic operations, enabling declarative iteration and parallel execution.
- **Type Inference**: Variable and expression types are inferred, so explicit type declarations are often unnecessary.
- **Immutable**: Variables are immutable unless specified, promoting safer code and easier reasoning.
- **Rich Text**: Mech programs support rich formatting and literate programming through Mechdown, a Markdown dialect.

Mech also supports a number of other features, such as pattern matching,state machines, comprehensions, and relational joins.

Here are a few more examples of Mech code to give you a sense of the language. First, the classic factorial program expressed in Mech:

<p align="center">
  <img width="500px" src="https://gitlab.com/mech-lang/assets/-/raw/ea310ad7a32025d34e6a60dc155a520266bd42b0/images/code/factorial.png" alt='The classic factorial program expressed in Mech.'>
  <a href="https://mech-lang.org/post/2026-05-11-version-0.3/#48174767513779710">Working Example</a>>
</p>

Pattern matching is a prominent feature throughout Mech. Here is an example of pattern matching on arrays:

<p align="center">
  <img width="500px" src="https://gitlab.com/mech-lang/assets/-/raw/ea310ad7a32025d34e6a60dc155a520266bd42b0/images/code/pattern-array.png" alt='A pattern match on arrays.'>
</p>

State machines are Mech's way of expressing protocols, processes, and asynchronous computations. Here is an example of a simple state machine that counts down from a given number:

<p align="center">
  <img width="500px" src="https://gitlab.com/mech-lang/assets/-/raw/ea310ad7a32025d34e6a60dc155a520266bd42b0/images/code/countdown.png" alt='A simple state machine that counts down from a given number.'>
</p>

Mech also supports comprehensions and relational joins. Here is an example of a comprehension that finds the friends of friends in a social graph by joining two generators over a common variable:

<p align="center">
  <img width="500px" src="https://gitlab.com/mech-lang/assets/-/raw/ea310ad7a32025d34e6a60dc155a520266bd42b0/images/code/friends-of-friends.png" alt='A comprehension that finds the friends of friends in a social graph.'>
</p>

## Run Mech

There are [several ways](https://docs.mech-lang.org/getting-started/install.html) to get Mech running on your machine, which you can find in the documentation. But the easiest way to get started is to use the online REPL at [try.mech-lang.org](https://try.mech-lang.org).

## Documentation

New to Mech? Start with [Learn Mech in Fifteen Minutes](https://docs.mech-lang.org/guides/mech-in-fifteen-minutes.html).

Comprehensive documentation is available at [docs.mech-lang.org](https://docs.mech-lang.org) and open-sourced on [GitHub](https://github.com/mech-lang/mech/tree/main/docs).

## Community

The Mech community gathers at a few places around the Internet:

- [Discord](https://discord.gg/asqP25NNTH) - for live chat
- [GitHub](https://github.com/mech-lang) - for code and issues
- [YouTube](https://www.youtube.com/@MechLang) - for video tutorials
- [Reddit](https://www.reddit.com/r/mechlang/) - for help and general discussion
- [Mailing List](https://groups.google.com/g/mechtalk) - for dev discussion

Feel free to stop by and introduce yourself -- we're happy to meet new users and answer questions! 

## Project Roadmap

Mech `v0.3` is currently **beta** status, meaning while many intended features are implemented and working well, other features are incomplete, not tested, or undocumented.

A Brief Roadmap:

- ☑️ [v0.1](https://github.com/mech-lang/mech/tree/v0.1-beta) - proof of concept system - minimum viable language implementation
- ☑️ [v0.2](https://github.com/mech-lang/mech/tree/v0.2-beta) - data specification - formulas, defining and manipulating data
- 📍 [v0.3](https://github.com/mech-lang/mech/tree/v0.3-beta) - program specification - functions, modules, state machines, Mika
- ☐ v0.4 - system specification - tools, distributed programs, capabilities

For more details, read the [ROADMAP](https://docs.mech-lang.org/design/ROADMAP.html).

## Notice

Mech should be considered unstable and therefore unfit for use in critical systems until `v1.0` is released.

## License

Licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0).
