const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const context = { document: { addEventListener() {} } };
const source = fs.readFileSync('frontend/assets/tasks.js', 'utf8');
vm.runInNewContext(source, context, { filename: 'tasks.js' });

function parse(raw) { return Array.from(context.parseTaskArgs(raw)); }
assert.deepEqual(parse('--path "D:\\My Project\\data.csv" --dry-run'),
  ['--path', 'D:\\My Project\\data.csv', '--dry-run']);
assert.deepEqual(parse('one "" two'), ['one', '', 'two']);
assert.deepEqual(parse('"quoted \\"text\\"" plain'), ['quoted "text"', 'plain']);
assert.deepEqual(parse('"C:\\Program Files\\Python\\python.exe"'),
  ['C:\\Program Files\\Python\\python.exe']);
assert.throws(() => parse('"unclosed'), /引号/);
console.log('task argument parser tests passed');