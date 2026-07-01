import { z } from 'zod';

export function zodToA2uiJsonSchema(schema: z.ZodTypeAny): Record<string, unknown> {
  const def = schema._def;

  if (def.typeName === z.ZodFirstPartyTypeKind.ZodObject) {
    const shape = def.shape() as Record<string, z.ZodTypeAny>;
    const properties: Record<string, unknown> = {};
    const required: string[] = [];
    for (const [key, fieldSchema] of Object.entries(shape)) {
      properties[key] = zodToA2uiJsonSchema(fieldSchema);
      if (!(fieldSchema instanceof z.ZodOptional)) {
        required.push(key);
      }
    }
    return { type: 'object', properties, required };
  }

  if (def.typeName === z.ZodFirstPartyTypeKind.ZodString) return { type: 'string' };
  if (def.typeName === z.ZodFirstPartyTypeKind.ZodNumber) return { type: 'number' };
  if (def.typeName === z.ZodFirstPartyTypeKind.ZodBoolean) return { type: 'boolean' };
  if (def.typeName === z.ZodFirstPartyTypeKind.ZodArray) {
    return { type: 'array', items: zodToA2uiJsonSchema(def.type as z.ZodTypeAny) };
  }
  if (def.typeName === z.ZodFirstPartyTypeKind.ZodEnum) {
    return { type: 'string', enum: (def as z.ZodEnumDef).values };
  }
  if (def.typeName === z.ZodFirstPartyTypeKind.ZodOptional) {
    return zodToA2uiJsonSchema((def as z.ZodOptionalDef).innerType);
  }

  return {};
}
