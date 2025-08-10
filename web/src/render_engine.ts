import * as glMatrix from "gl-matrix";

const text_vs_standard = `
	attribute vec3 vertpos;
	attribute vec2 vertuv;
	uniform mat4 P;
	uniform mat4 M;
	varying lowp vec2 UV;
	void main(void) {
		gl_Position = P * M * vec4(vertpos, 1.0);
		UV = vertuv;
	}
`;

const text_fs_solid = `
	precision highp float;
	varying vec2 UV;

	uniform sampler2D tex;

	void main() {
		gl_FragColor = texture2D(tex, UV);
	}
`;

// FFFFFF keying
const text_fs_color_keyed = `
	precision highp float;
	varying vec2 UV;

	uniform sampler2D tex;

	void main() {
		vec4 color = texture2D(tex, UV).rgba;
		if(color.r == 1.0 && color.b == 1.0 && color.a == 1.0) {
			color.a = 0.0;
		}

		gl_FragColor = color;
	}
`;

function loadShader(gl: WebGL2RenderingContext, type: number, source: string) {
	const shader = gl.createShader(type);
	if (!shader) {
		console.log("gl.createShader failed");
		return null;
	}

	gl.shaderSource(shader, source);
	gl.compileShader(shader);

	if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
		console.log("Failed to compile shader: " + gl.getShaderInfoLog(shader));
		gl.deleteShader(shader);
		return null;
	}

	return shader;
}

function initShaderProgram(gl: WebGL2RenderingContext, source_vs: string, source_fs: string) {
	const vertexShader = loadShader(gl, gl.VERTEX_SHADER, source_vs);
	const fragmentShader = loadShader(gl, gl.FRAGMENT_SHADER, source_fs);

	if (!vertexShader || !fragmentShader)
		return null;

	const shaderProgram = gl.createProgram();
	if (!shaderProgram)
		return null;

	gl.attachShader(shaderProgram, vertexShader);
	gl.attachShader(shaderProgram, fragmentShader);
	gl.linkProgram(shaderProgram);
	if (!gl.getProgramParameter(shaderProgram, gl.LINK_STATUS)) {
		console.log("Failed to initialize shader program: " + gl.getProgramInfoLog(shaderProgram));
		return null;
	}

	return shaderProgram;
}

class RObject {
	vao!: WebGLVertexArrayObject;
	position!: WebGLBuffer;
	uv!: WebGLBuffer;
}

function initBufferQuad(gl: WebGL2RenderingContext): RObject {
	let vao = gl.createVertexArray();
	gl.bindVertexArray(vao);

	const positionBuffer = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
	const positions = [
		0.0, 0.0,
		0.0, 1.0,
		1.0, 1.0,
		0.0, 0.0,
		1.0, 1.0,
		1.0, 0.0
	];

	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(positions), gl.STATIC_DRAW);
	gl.enableVertexAttribArray(0);
	gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

	const uvs = [
		0.0, 0.0,
		0.0, 1.0,
		1.0, 1.0,
		0.0, 0.0,
		1.0, 1.0,
		1.0, 0.0
	];

	const uvBuffer = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, uvBuffer);
	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(uvs), gl.STATIC_DRAW);
	gl.enableVertexAttribArray(1);
	gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 0, 0);

	let robject = new RObject();
	robject.vao = vao!;
	robject.position = positionBuffer!;
	robject.uv = uvBuffer!;
	return robject;
}

class Shader {
	program: WebGLProgram | null;
	uniform_p;
	uniform_m;

	constructor(gl: WebGL2RenderingContext, source_vs: string, source_fs: string) {
		this.program = initShaderProgram(gl, source_vs, source_fs);
		if (!this.program)
			return;
		this.uniform_p = this.createUniform(gl, "P");
		this.uniform_m = this.createUniform(gl, "M");
	}

	createUniform(gl: WebGL2RenderingContext, name: string) {
		return gl.getUniformLocation(this.program!, name);
	}

	bind(gl: WebGL2RenderingContext) {
		gl.useProgram(this.program);
	}

	setM(gl: WebGL2RenderingContext, m: any) {
		gl.uniformMatrix4fv(this.uniform_m!, false, m);
	}

	setP(gl: WebGL2RenderingContext, p: any) {
		gl.uniformMatrix4fv(this.uniform_p!, false, p);
	}
}

export class Texture {
	texture!: WebGLTexture;
	width: number = 0;
	height: number = 0;
}

export interface RenderEngineParams {
	color_keyed: boolean,
	canvas: HTMLCanvasElement,
}

export class RenderEngine {
	params: RenderEngineParams;
	gl: WebGL2RenderingContext;
	projection = glMatrix.mat4.create();
	display_scale: number = 1.0;
	buffer_quad: RObject;
	shader_solid: Shader;
	shader_color_keyed: Shader;

	constructor(params: RenderEngineParams) {
		this.params = params;
		const gl = params.canvas.getContext("webgl2",
			{
				antialias: false,
				alpha: params.color_keyed,
				premultipliedAlpha: false
			});

		if (!gl) {
			alert("WebGL2 not supported.\nPlease enable WebGL in browser settings or update your graphics card/browser.");
			throw new Error("WebGL2 not supported");
		}

		this.gl = gl;

		this.buffer_quad = initBufferQuad(this.gl);
		this.shader_solid = new Shader(this.gl, text_vs_standard, text_fs_solid);
		this.shader_color_keyed = new Shader(this.gl, text_vs_standard, text_fs_color_keyed);

		this.gl.disable(this.gl.DEPTH_TEST);
		this.gl.enable(this.gl.BLEND);
		this.gl.blendFunc(this.gl.SRC_ALPHA, this.gl.ONE_MINUS_SRC_ALPHA);
	}

	clear(r: number, g: number, b: number, a: number) {
		this.gl.clearColor(r, g, b, a);
		this.gl.clear(this.gl.COLOR_BUFFER_BIT);
	}

	viewportFullscreen() {
		this.gl.viewport(0, 0, this.params.canvas.width, this.params.canvas.height);
	}

	setOrtho(left: number, right: number, bottom: number, top: number) {
		glMatrix.mat4.ortho(this.projection, left, right, bottom, top, -1.0, 1.0);
	}

	drawRect(shader: Shader, texture: Texture, pos_x: number, pos_y: number, width: number, height: number) {
		const model = glMatrix.mat4.create();
		glMatrix.mat4.translate(model, model, [pos_x, pos_y, 0.0]);
		glMatrix.mat4.scale(model, model, [width, height, 1.0]);

		shader.bind(this.gl);
		shader.setP(this.gl, this.projection);
		shader.setM(this.gl, model)

		this.gl.bindVertexArray(this.buffer_quad.vao);
		this.gl.bindTexture(this.gl.TEXTURE_2D, texture.texture);
		this.gl.drawArrays(this.gl.TRIANGLES, 0, 6);
	}

	loadTextureImage(url: string, callback: (texture: Texture) => void) {
		let gl = this.gl;

		const image = new Image();

		image.onload = function () {
			const tex = gl.createTexture();

			if (!tex)
				return;

			gl.bindTexture(gl.TEXTURE_2D, tex);
			gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);

			let tex_obj = new Texture();
			tex_obj.texture = tex;
			tex_obj.width = image.width;
			tex_obj.height = image.height;

			console.log("Texture " + url + " loaded");
			callback(tex_obj);
		};

		image.src = url;
	}
}

