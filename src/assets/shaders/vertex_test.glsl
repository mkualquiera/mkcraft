#version 330 core
layout (location = 0) in vec3 pos;
layout (location = 1) in vec2 uv;
layout (location = 2) in ivec2 mat;
layout (location = 3) in vec4 color;
layout (location = 4) in vec3 light;
flat out ivec2 fragMaterialId;
out vec4 fragColor;
out vec2 fragUV;
out vec3 fragLight;
out vec3 worldPos;
out vec4 glPos;

uniform mat4 mvp;
void main() {
  
  gl_Position = mvp * vec4(pos.x, pos.y, pos.z, 1.0);
  // PS1 effect supposedly
  //gl_Position.xy = gl_Position.xy / gl_Position.w;
  //gl_Position.xy = round(gl_Position.xy * vec2(100.0, 75.0)) / vec2(100.0, 75.0);
  //gl_Position.xy = gl_Position.xy * gl_Position.w;
  worldPos = pos;
  fragColor = color;
  fragUV = uv * gl_Position.w;
  glPos = gl_Position;
  fragMaterialId = mat;
  fragLight = light;
}