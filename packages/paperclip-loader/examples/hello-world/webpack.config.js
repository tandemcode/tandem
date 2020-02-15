const path = require("path");
const webpack = require("webpack");

const HtmlWebpackPlugin = require("html-webpack-plugin");

/*
 * We've enabled HtmlWebpackPlugin for you! This generates a html
 * page for you when you compile webpack, which will make you start
 * developing and prototyping faster.
 *
 * https://github.com/jantimon/html-webpack-plugin
 *
 */

module.exports = {
  mode: "development",
  entry: "./src/index.tsx",

  output: {
    filename: "[name].js",
    path: path.resolve(__dirname, "dist")
  },
  devtool: false,

  plugins: [new HtmlWebpackPlugin()],

  module: {
    rules: [
      {
        test: /\.(ts|tsx)?$/,
        loader: "ts-loader",
        include: [path.resolve(__dirname, "src")],
        exclude: [/node_modules/]
      },
      {
        test: /\.pc$/,
        loader: "paperclip-loader",
        include: [path.resolve(__dirname, "src")],
        exclude: [/node_modules/],
        options: {
          compiler: "paperclip-react-compiler"
        }
      }
    ]
  },

  resolve: {
    extensions: [".tsx", ".ts", ".js"]
  }
};
