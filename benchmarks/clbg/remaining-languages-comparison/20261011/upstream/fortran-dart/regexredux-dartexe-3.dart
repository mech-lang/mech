/// The Computer Language Benchmarks Game
/// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
///
/// regex-dna program contributed by Jos Hirth, based on the JavaScript version
/// which was created by Jesse Millikan, jose fco. gonzalez, and Matthew Wilson
/// parallelism added by Dwayne Slater
/// migrated by Alexander Fyodorov

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:isolate';

// Set up an isolate for each CPU asynchronously to accept tasks

final taskPorts = List<Completer<SendPort>>.generate(
  Platform.numberOfProcessors,
  (i) => Completer<SendPort>.sync(),
);

final taskRecvPorts = List<RawReceivePort>.generate(
  Platform.numberOfProcessors,
  (i) => RawReceivePort(taskPorts[i].complete),
);

final isolates = List<Future<Isolate>>.generate(
  Platform.numberOfProcessors,
  (i) => Isolate.spawn(taskRunner, taskRecvPorts[i].sendPort),
);

void main() {
  // Start up the isolates while building data from stdin
  isolates;

  stdin.transform(Latin1Decoder()).join().then(regexAllTheThings);
}

// Isolate entry point, just runs given tasks
void taskRunner(SendPort port) {
  final recvPort = RawReceivePort((task) {
    task[0](task[1]);
  });
  port.send(recvPort.sendPort);
}

late String regexTaskFullText;

// Task that initializes this isolate with a copy of the text from stdin
void initStaticFullText(String fullText) {
  regexTaskFullText = fullText;
}

// Core regex task
// Params are: [pattern: String, index: int, replyPort: SendPort]
void runRegexTask(List params) {
  params[2].send([
    params[1],
    "${params[0]} ${RegExp(params[0], caseSensitive: false).allMatches(regexTaskFullText).length}",
  ]);
}

void regexAllTheThings(String fullText) {
  var lengthA, lengthB, lengthC, regexp, replacements;
  final List<Completer<String>> taskResult;

  regexp = [
    'agggtaaa|tttaccct',
    '[cgt]gggtaaa|tttaccc[acg]',
    'a[act]ggtaaa|tttacc[agt]t',
    'ag[act]gtaaa|tttac[agt]ct',
    'agg[act]taaa|ttta[agt]cct',
    'aggg[acg]aaa|ttt[cgt]ccct',
    'agggt[cgt]aa|tt[acg]accct',
    'agggta[cgt]a|t[acg]taccct',
    'agggtaa[cgt]|[acg]ttaccct',
  ];

  replacements = [
    'tHa[Nt]',
    '<4>',
    'aND|caN|Ha[DS]|WaS',
    '<3>',
    'a[NSt]|BY',
    '<2>',
    '<[^>]*>',
    '|',
    '\\|[^|][^|]*\\|',
    '-'
  ];

  lengthA = fullText.length;

  // still ridiculously slow with r21658
  fullText = fullText.replaceAll(RegExp('^>.*\n|\n', multiLine: true), '');

  lengthB = fullText.length;

  // Completes with the result of each regex task
  taskResult = List<Completer<String>>.generate(
    regexp.length,
    (i) => Completer<String>.sync(),
  );

  // Propagates each task result to their proper destination
  final taskRecv =
      RawReceivePort((result) => taskResult[result[0]].complete(result[1]));

  // Send a copy of the full text to each task runner
  // fullText can get modified later on, so we take _this_ version of fullText
  // to avoid possible "race conditions"
  {
    final sentFullText = fullText;
    taskPorts.forEach((completer) {
      completer.future.then((port) {
        port.send([initStaticFullText, sentFullText]);
      });
    });
  }

  // Sends each task to the task isolate for processing
  for (var i = 0; i < regexp.length; ++i) {
    final payload = [regexp[i], i, taskRecv.sendPort];
    taskPorts[i % taskPorts.length].future.then((port) {
      port.send([
        runRegexTask,
        payload,
      ]);
    });
  }

  for (var i = -1; i < replacements.length - 1;) {
    fullText =
        fullText.replaceAll(RegExp(replacements[++i]), replacements[++i]);
  }

  lengthC = fullText.length;

  // Waits for the result of each task to be printed, then prints the final
  // lengths, then cleans up
  Future.forEach(
          taskResult, (Completer<String> result) => result.future.then(print))
      .then((_) {
    print('\n$lengthA\n$lengthB\n$lengthC');
    // Clean up our receive ports so Dart can exit gracefully
    taskRecv.close();
    taskRecvPorts.forEach((port) => port.close());
  });
}
