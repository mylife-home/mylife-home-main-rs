import path from 'path';
import glob from 'glob';
import { Configuration } from 'webpack';
import { merge } from 'webpack-merge';
import MiniCssExtractPlugin from 'mini-css-extract-plugin';
import CopyPlugin from 'copy-webpack-plugin';
import HtmlWebpackPlugin from 'html-webpack-plugin';
import TerserPlugin from 'terser-webpack-plugin';
import PurgecssPlugin from 'purgecss-webpack-plugin';

type ConfigurationByMode = { [mode: string]: Configuration; };

export default (env: any, argv: any) => {
  const mode = argv.mode === 'development' ? 'dev' : 'prod';

  const babelOptions = {
    presets: [
      ['@babel/preset-env', { targets: { browsers: ['defaults', 'ios_saf >= 9'] } }],
      '@babel/preset-react',
    ],
    cacheDirectory: true,
  };

  const basePath = path.resolve('..');
  const sourcePath = path.join(basePath, 'src');

  const styleLoader = mode === 'prod' ? MiniCssExtractPlugin.loader : 'style-loader';

  const base: Configuration = {
    entry: path.join(sourcePath, 'app/main'),

    output: {
      path: path.join(basePath, 'dist'),
    },

    target: 'browserslist:defaults, ios_saf >= 9',

    resolve: {
      extensions: ['.wasm', '.mjs', '.js', '.ts', '.tsx', '.json'],

      // https://preactjs.com/guide/v10/getting-started#aliasing-react-to-preact
      alias: {
        "react": "preact/compat",
        "react-dom/test-utils": "preact/test-utils",
        "react-dom": "preact/compat",
      },
    },

    module: {
      rules: [
        {
          test: /\.ts(x?)$/,
          use: [
            { loader: 'babel-loader', options: babelOptions },
            { loader: 'ts-loader', options: { configFile: path.join(basePath, 'tsconfig.json') } },
          ],
        },
        { test: /\.(m|c?)js$/, use: [{ loader: 'babel-loader', options: babelOptions }] },
        { test: /\.css$/, use: [styleLoader, 'css-loader'] },
        { test: /\.scss$/, use: [styleLoader, 'css-loader', 'sass-loader'] },
        { test: /\.(png|jpg|gif|svg|eot|woff|woff2|ttf|ico)$/, use: ['file-loader'] },
      ],
    },

    plugins: [
      new CopyPlugin({
        patterns: [
          { from: path.join(basePath, 'static'), to: '' },
        ],
      }),
      new HtmlWebpackPlugin({ template: path.join(sourcePath, 'index.html') })
    ]
  };

  const modes: ConfigurationByMode = {
    dev: {
      mode: 'development',
      devtool: 'eval',
      optimization: {
        splitChunks: {
          cacheGroups: {
            vendor: {
              test: /[\\/]node_modules[\\/]/,
              name: 'vendors',
              chunks: 'all',
            },
          },
        },
      },
    },
    prod: {
      mode: 'production',
      devtool: 'nosources-source-map',
      optimization: {
        minimizer: [
          new TerserPlugin({
            terserOptions: {
              keep_classnames: true,
              keep_fnames: true,
            },
          }),
        ],
      },
      plugins: [
        new MiniCssExtractPlugin(),
        new PurgecssPlugin({ paths: glob.sync(`${sourcePath}/**/*`, { nodir: true }) }),
      ],
    },
  };

  const modeConfig = modes[mode];
  if (!modeConfig) {
    throw new Error(`Unsupported mode: '${mode}`);
  }

  const devServerConfig: Configuration = {};

  if (mode === 'dev') {
    (devServerConfig as any).devServer = {
      host: '0.0.0.0',
      port: 8101,
      allowedHosts: 'all',
      proxy: [
        { context: ['/resources', '/repository'], target: 'http://localhost:8001' },
        { context: ['/websocket'], target: 'ws://localhost:8001', ws: true },
      ],
    };
  }

  return merge(base, modeConfig, devServerConfig);
};
