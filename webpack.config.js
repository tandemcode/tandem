module.exports = {
    entry: './src/entry.js',
    output: {
        path: __dirname,
        filename: 'public/bundle.js'
    },
    resolve: {
      modulesDirectories: ['node_modules', 'bower_components'],
      extensions: ['', '.jsx', '.js']
    },
    publicPath: 'public',
    module: {
        loaders: [
            { test: /\.css$/, loader: 'style!css' },
            {
              test: /\.jsx?$/,
              exclude: /(node_modules|bower_components)/,
              loader: 'babel', // 'babel-loader' is also a legal name to reference
              query: {
                presets: ['react', 'es2015']
              }
            }
        ]
    }
};
