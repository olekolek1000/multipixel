const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
var ProgressPlugin = require('progress-webpack-plugin')
const CopyPlugin = require("copy-webpack-plugin");
var webpack = require('webpack');
const isProduction = process.env.NODE_ENV == "production";

const connect_url = process.env.CONNECT_URL ? process.env.CONNECT_URL : "ws://127.0.0.1:59900";

let commit_hash = require('child_process')
	.execSync('git rev-parse --short HEAD')
	.toString()
	.trim();

let commit_branch = require('child_process')
	.execSync('git rev-parse --abbrev-ref HEAD')
	.toString()
	.trim();


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
		new webpack.DefinePlugin({
			__COMMIT_HASH__: JSON.stringify(commit_hash),
			__COMMIT_BRANCH__: JSON.stringify(commit_branch),
			__CONNECT_URL__: JSON.stringify(connect_url)
		}),
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
