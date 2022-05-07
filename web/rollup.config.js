import watch from "rollup-plugin-watch";
import resolve from 'rollup-plugin-node-resolve';

export default {
  input: 'lib/index.js',
  output: {
    file: 'dist/bundle.js',
    format: 'iife'
  }, plugins: [
    watch({dir: "lib"}),
    resolve({
      jsnext: true,
      main: true,
      module: true
    })
  ]
};
