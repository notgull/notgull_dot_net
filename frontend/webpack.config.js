// GNU AGPL v3 License

module.exports = {
    entry: './src/index.ts',
    module: {
      rules: [
        {
          test: /\.tsx?$/,
          use: 'ts-loader',
          exclude: /node_modules/,
        },
      ],
    },
    resolve: {
      extensions: ['.tsx', '.ts', '.js'],
    },
    output: {
      filename: 'notgull.js',
    },
    externals: {
      "axios": "axios",
      "preact": "preact",
    },
};
  