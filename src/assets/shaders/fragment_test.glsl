#version 330 core

in vec4 fragColor;
in vec2 fragUV;
in vec3 fragLight;
in vec3 worldPos;
in vec4 glPos;
flat in ivec2 fragMaterialId;

uniform sampler2D terrainTexture;
uniform float time;
uniform vec3 cameraPos;
uniform vec3 cursorPos;

float dither4x4(vec2 position) {
    int x = int(mod(position.x, 4.0));
    int y = int(mod(position.y, 4.0));
    
    // 4x4 Bayer matrix
    int index = x + y * 4;
    int dither[16] = int[](0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5);
    
    return float(dither[index]) / 16.0;
}

// Simple hash function
float hash(vec2 p) {
    p = fract(p * vec2(234.34, 435.345));
    p += dot(p, p + 34.23);
    return fract(p.x * p.y);
}

// Simple noise
float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    
    float a = hash(i);
    float b = hash(i + vec2(1.0, 0.0));
    float c = hash(i + vec2(0.0, 1.0));
    float d = hash(i + vec2(1.0, 1.0));
    
    vec2 u = f * f * (3.0 - 2.0 * f); // Smooth interpolation
    
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// 3D noise function
float hash3(vec3 p) {
    p = fract(p * vec3(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yxz + 33.33);
    return fract((p.x + p.y) * p.z);
}

float noise3(vec3 p) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    
    // Sample all 8 corners of the cube
    float a = hash3(i);
    float b = hash3(i + vec3(1.0, 0.0, 0.0));
    float c = hash3(i + vec3(0.0, 1.0, 0.0));
    float d = hash3(i + vec3(1.0, 1.0, 0.0));
    float e = hash3(i + vec3(0.0, 0.0, 1.0));
    float f2 = hash3(i + vec3(1.0, 0.0, 1.0));
    float g = hash3(i + vec3(0.0, 1.0, 1.0));
    float h = hash3(i + vec3(1.0, 1.0, 1.0));
    
    // Smooth interpolation
    vec3 u = f * f * (3.0 - 2.0 * f);
    
    // Trilinear interpolation
    return mix(mix(mix(a, b, u.x), mix(c, d, u.x), u.y),
               mix(mix(e, f2, u.x), mix(g, h, u.x), u.y), u.z);
}

out vec4 final_color;
void main() {

  float cursorStartX = cursorPos.x - 0.05;
  float cursorEndX = cursorPos.x + 1.05;
  float cursorStartY = cursorPos.y - 0.05;
  float cursorEndY = cursorPos.y + 1.05;
  float cursorStartZ = cursorPos.z - 0.05;
  float cursorEndZ = cursorPos.z + 1.05;
  bool isCursorInBlock = (worldPos.x >= cursorStartX && worldPos.x <= cursorEndX) &&
                         (worldPos.y >= cursorStartY && worldPos.y <= cursorEndY) &&
                         (worldPos.z >= cursorStartZ && worldPos.z <= cursorEndZ);

  vec3 flooredPos = floor(worldPos * 16.0) / 16.0;

  // Convert materialId to a texture coordinate by 
  // dividing by the number of materials (16)
  vec2 matCoord = vec2(fragMaterialId) / 16.0;
  // Add uv downscaled by 16

  vec2 UV = fragUV.xy / glPos.w;

  if (fragLight.r < fragLight.b) {
      vec3 flooredPos = floor((worldPos + 0.001) * 16.0) / 16.0 + 0.1321;

      float noiseT = noise3(vec3(time*0.1) + flooredPos);
    
      float offx = noise3(flooredPos*2.0 + vec3(noiseT) + vec3(time * 0.1, 0.0, 0.0)) * 0.1;
      float offy = noise3(flooredPos*2.0 + vec3(noiseT) + vec3(0.0, time * 0.2, 0.0)) * 0.1;

      offx = floor(offx * 16.0) / 16.0;
      offy = floor(offy * 16.0) / 16.0;

      UV = UV + vec2(offx, offy);

      if (UV.x < 0.0) {
       UV.x = 0;
      }
      if (UV.y < 0.0) {
       UV.y = 0;
      }
      if (UV.x > 0.95) {
       UV.x = 0.95;
      }
      if (UV.y > 0.95) {
       UV.y = 0.95;
      }
  }

  vec2 wrappedUV = vec2(mod(UV.x,1), mod(UV.y,1));
  vec2 texCoord = matCoord + wrappedUV / 16.0;
  // Sample the texture
  
  vec4 sampledColor = texture(terrainTexture, texCoord);
  
  // Only apply darkness to RGB, preserve alpha
  vec3 darkness = fragLight.rgb - vec3(1.0, 1.0, 1.0);

  if (fragLight.r < fragLight.b) {
    if (abs(worldPos.y - floor(worldPos.y)) < 0.001) {
    vec3 flooredPos = floor((worldPos + 0.001) * 16.0) / 16.0 + 0.1321;
    float samplea = noise3(vec3(flooredPos.x*2, flooredPos.z*2, 1*time)) * 1;
    float sampleb = noise3(vec3(flooredPos.x*2, flooredPos.z*2, 1*(time+100))) *1;
    float sample = (samplea + sampleb) * 0.5;
    sample = 0.5 - abs(sample);
    if (sample < 0.0) {
        sample = 0;
    }
    darkness.rgb += sample * vec3(1, 0.5, 0.4);
    }
  }

  //darkness = darkness - vec3(0.1, 0.2, 0.3); // Slightly reduce darkness
  //vec3 ambientLight = vec3(0.51, 0.86, 0.9)*2.0;
  //darkness = darkness - (vec3(1.0,1.0,1.0)- ambientLight);
  
  if (sampledColor.r == sampledColor.g && sampledColor.g == sampledColor.b) {
      // Grayscale case
      vec3 offset = (sampledColor - vec4(1.0, 1.0, 1.0, 0.0)).rgb;
      final_color = vec4(fragColor.rgb + offset + darkness, sampledColor.a);
  } else {
      // Colored texture case - preserve the original alpha!
      final_color = vec4(sampledColor.rgb + darkness, sampledColor.a);
  }

  // special handling for water
  if (fragMaterialId.x == 15 && fragMaterialId.y == 13) {
      // Water material, apply special color
      //if (dither4x4(vec2((fragUV.x + time)*5, (fragUV.y + time)*5)) > 0.5) {
      //  discard;
      //}

      
      // === TURBULENT WAVES (Base layer) ===
      float freq1 = 0.1;
      float freq2 = 0.8;
      float freq3 = 4.6;

      float angle1 = noise3(vec3(flooredPos.x / 80.0f, flooredPos.z / 80.0f, time*0.0002)) * 6.28;
      float angle2 = noise3(vec3(flooredPos.x / 40.0f, flooredPos.z / 40.0f, time*0.005)) * (6.28/2.0) + angle1; 
      float angle3 = noise3(vec3(flooredPos.x / 20.0f, flooredPos.z / 20.0f, time*0.01)) * (6.28/4.0) + angle2;

      float freq1x = cos(angle1) * freq1;
      float freq1z = sin(angle1) * freq1;

      // Large slow waves
      float layer1 = cos(flooredPos.x * freq1x + flooredPos.z * freq1z + time * 0.4) * 0.3;

      float freq2x = cos(angle2) * freq2;
      float freq2z = sin(angle2) * freq2;

      // Medium detail
      float layer2 = sin(flooredPos.x * freq2x + flooredPos.z * freq2z + time * 0.6) * 0.2;

      float freq3x = cos(angle3) * freq3;
      float freq3z = sin(angle3) * freq3;

      // Fine detail
      float layer3 = cos(flooredPos.x * freq3x + flooredPos.z * freq3z + 
                         time *1.0) * 0.2;

      float waves = (layer1 + layer2 + layer3) * 1.0;

      // === COMBINE ===
      float origin = (waves) * 0.5 + 0.5;

      float distanceToCamera = length(flooredPos - cameraPos);

      float distanceBias = (smoothstep(0.0, 10.0, distanceToCamera) - 0.5) * 1; // Adjust ranges as needed

      float threshold = ((origin + dither4x4(vec2(worldPos.x*16.0, worldPos.z*16.0)))) + distanceBias;

      if (threshold < 1.0) {
          discard; 
      }

      //final_color.rgb = 1.0*vec3(layer1* final_color.b, layer1* final_color.b, final_color.b); // Blue-green tint

      vec3 darkColor = vec3(0.05, 0.17, 0.10); // Dark blue-green color
      vec3 lightColor = vec3(0.45, 0.71, 0.73); // Lighter blue-green color
      //vec3 lightColor = vec3(122.0/255.0, 64.0/255.0, 30.0/255.0);

      final_color.rgb = mix(darkColor, lightColor, layer1 * 0.5 + 0.5);
      
      final_color.a = 1.0;
  }

  if (final_color.a < 0.1) {
      discard; // Discard fragments with very low alpha
  }

  if (isCursorInBlock) {
      // Highlight the block under the cursor
      final_color.rgb -= vec3(0.1, 0.1, 0.1);
  }

  //final_color = vec4(fragUV, 0.0, 1.0);
}