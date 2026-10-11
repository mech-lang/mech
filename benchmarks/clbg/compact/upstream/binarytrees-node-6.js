/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Léo Sarrazin
   multi thread by Andrey Filatkin
*/

const { Worker, isMainThread, parentPort, workerData } = require('worker_threads');

if (isMainThread) {
    mainThread();
} else {
    workerThread(workerData);
}

async function mainThread() {
    const maxDepth = Math.max(6, parseInt(process.argv[2]));

    const stretchDepth = maxDepth + 1;
    const check = itemCheck(bottomUpTree(stretchDepth));
    console.log(`stretch tree of depth ${stretchDepth}\t check: ${check}`);

    const longLivedTree = bottomUpTree(maxDepth);

    const tasks = [];
    for (let depth = 4; depth <= maxDepth; depth += 2) {
        const iterations = 1 << maxDepth - depth + 4;
        tasks.push({iterations, depth});
    }

    const results = await runTasks(tasks);
    for (const result of results) {
        console.log(result);
    }

    console.log(`long lived tree of depth ${maxDepth}\t check: ${itemCheck(longLivedTree)}`);
}

function workerThread({iterations, depth}) {
    parentPort.postMessage({
        result: work(iterations, depth)
    });
}

function runTasks(tasks) {
    return new Promise(resolve => {
        const results = [];
        let tasksSize = tasks.length;

        for (let i = 0; i < tasks.length; i++) {
            const worker = new Worker(__filename, {workerData: tasks[i]});

            worker.on('message', message => {
                results[i] = message.result;
                tasksSize--;
                if (tasksSize === 0) {
                    resolve(results);
                }
            });
        }
    });
}

function work(iterations, depth) {
    let check = 0;
    for (let i = 0; i < iterations; i++) {
        check += itemCheck(bottomUpTree(depth));
    }
    return `${iterations}\t trees of depth ${depth}\t check: ${check}`;
}

function TreeNode(left, right) {
    return {left, right};
}

function itemCheck(node) {
    if (node.left === null) {
        return 1;
    }
    return 1 + itemCheck(node.left) + itemCheck(node.right);
}

function bottomUpTree(depth) {
    return depth > 0
        ? new TreeNode(bottomUpTree(depth - 1), bottomUpTree(depth - 1))
        : new TreeNode(null, null);
}
