#version 330 core

in vec4 fragColor;
in vec2 fragUV;
in vec3 fragLight;
in vec3 worldPos;
flat in ivec2 fragMaterialId;

uniform sampler2D terrainTexture;
uniform float time;
uniform vec3 cameraPos;

out vec4 final_color;
void main() {
    vec2 UV = fragUV.xy;

    vec2 matCoord = vec2(fragMaterialId) / 16.0;

    vec2 wrappedUV = vec2(mod(UV.x,1), mod(UV.y,1));
    vec2 texCoord = matCoord + wrappedUV / 16.0;   

    vec2 shadowCoord = texCoord - 1/(16.0*8.0);

    vec4 sampledColor = texture(terrainTexture, texCoord);
    vec4 shadowColor = texture(terrainTexture, shadowCoord);

    vec3 darkness = fragLight.rgb - vec3(1.0, 1.0, 1.0);

    if (sampledColor.a > 0.5) {
        final_color.rgba = sampledColor.rgba * fragColor.rgba;
    } else {
        float shadowAlpha = shadowColor.a;
        if (shadowAlpha < 0.5) {
            discard;
        } else {
            final_color.rgba = vec4(shadowColor.rgb * fragColor.rgb + darkness, fragColor.a);
        }
    }

    //final_color.rgba = vec4(1.0, 1.0, 1.0, 1.0);
}