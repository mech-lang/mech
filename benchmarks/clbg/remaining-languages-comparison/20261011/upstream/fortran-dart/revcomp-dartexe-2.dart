/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Contributed by James Wendel
   + null safety    
*/

import 'dart:io';

void main() {
  var src = "CGATMKRYVBHD";
  var dst = "GCTAKMYRBVDH";
  var tbl = List.filled(256, 0);
  var seq = <int>[];

  // Set up lookup table
  for (int i = 0; i < tbl.length; i++) tbl[i] = i;

  for (int i = 0; i < src.length; i++) {
    tbl[src.codeUnitAt(i)] = dst.codeUnitAt(i);
    tbl[src.toLowerCase().codeUnitAt(i)] = dst.codeUnitAt(i);
  }

  var buffer = List.filled(60, 0);
  var list = <int>[];
  bool commentLine = false;
  StringBuffer sbuf = new StringBuffer();

  stdin.listen((List<int> dataList) {
    // Loop over all the contents of the buffer so far
    for (int data in dataList) {
      // Check if this is a comment line (and that we aren't already on a comment line)
      if (data == 62 && !commentLine) {
        int count = 0;

        // Print the reverse components for the last block
        for (int g in list.reversed) {
          if (count == 60) {
            sbuf.write(new String.fromCharCodes(buffer.getRange(0, count)));
            sbuf.write('\n');
            count = 0;
          }
          buffer[count++] = g;
        }
        // Print any stragling data
        if (count > 0) {
          sbuf.write(new String.fromCharCodes(buffer.getRange(0, count)));
          sbuf.write('\n');
        }
        // Reset the data for the begining of a block of data
        list.clear();
        commentLine = true;
      }

      if (commentLine) {
        if (data == 10) {
          sbuf.write(new String.fromCharCodes(list));
          print(sbuf);
          sbuf = new StringBuffer();
          commentLine = false;
          list.clear();
        } else {
          list.add(data);
        }
      } else if (data != 10) {
        // Add the complement to the buffer
        list.add(tbl[data]);
      }
    }
  }).onDone(() {
    // Print out anything remaining in the buffers
    if (commentLine) {
      sbuf.write(new String.fromCharCodes(list));
    } else {
      int count = 0;
      for (int data in list.reversed) {
        if (count == 60) {
          sbuf.write(new String.fromCharCodes(buffer.getRange(0, count)));
          sbuf.write('\n');
          count = 0;
        }
        buffer[count++] = data;
      }
      if (count > 0) {
        sbuf.write(new String.fromCharCodes(buffer.getRange(0, count)));
      }
    }
    print(sbuf);
  });
}
