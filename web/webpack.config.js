const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
var ProgressPlugin = require('progress-webpack-plugin')
const CopyPlugin = require("copy-webpack-plugin");

const isProduction = process.env.NODE_ENV == "production";

const config = {
  entry: "./src/index.tsx",
  output: {
    path: path.resolve(__dirname, "dist"),
  },
  devServer: {
    open: true,
    host: "localhost",
  },
  plugins: [
    new HtmlWebpackPlugin({
      template: "src/index.html",
      favicon: "public/favicon.png"
    }),
    new CopyPlugin({
      patterns: [
        { from: "public", to: "public" },
      ],
    }),
    new ProgressPlugin(true)
  ],
  module: {
    rules: [
      {
        test: /\.(ts|tsx)$/i,
        loader: "ts-loader",
        exclude: ["/node_modules/"],
      },
      {
        test: /\.scss$/, use: [
          "style-loader",
          { loader: "css-modules-typescript-loader", options: {} },
          {
            loader: "css-loader", options: {
              url: false, modules: {
                localIdentName: '[local]--[hash:base64:6]'
              }
            }
          },
          "sass-loader"
        ]
      }
    ],
  },
  resolve: {
    extensions: [".tsx", ".ts", ".js"]
  },
};

module.exports = () => {
  if (isProduction) {
    config.mode = "production";
  } else {
    config.mode = "development";
  }
  return config;
};
