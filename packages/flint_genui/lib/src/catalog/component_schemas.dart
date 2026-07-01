/// JSON Schema definitions for all 40 Flint A2UI components.
/// These mirror the schemas stored in flint_a2ui.components.schema.
class FlintComponentSchemas {
  static const Map<String, Map<String, dynamic>> all = {
    // Layout
    'stack': {'type': 'object', 'properties': {'direction': {'type': 'string', 'enum': ['horizontal', 'vertical']}, 'gap': {'type': 'number'}}},
    'card': {'type': 'object', 'properties': {'title': {'type': 'string'}, 'elevated': {'type': 'boolean'}}},
    'grid': {'type': 'object', 'properties': {'columns': {'type': 'number'}, 'gap': {'type': 'number'}}},
    'split': {'type': 'object', 'properties': {'ratio': {'type': 'string'}}},
    'tabs': {'type': 'object', 'properties': {'items': {'type': 'array'}, 'defaultValue': {'type': 'string'}}, 'required': ['items']},
    'accordion': {'type': 'object', 'properties': {'items': {'type': 'array'}, 'allowMultiple': {'type': 'boolean'}}, 'required': ['items']},
    'scroll': {'type': 'object', 'properties': {'maxHeight': {'type': 'string'}}},
    'modal': {'type': 'object', 'properties': {'open': {'type': 'boolean'}, 'title': {'type': 'string'}}, 'required': ['open']},
    'drawer': {'type': 'object', 'properties': {'open': {'type': 'boolean'}, 'side': {'type': 'string', 'enum': ['left', 'right']}}, 'required': ['open']},

    // Data Display
    'data-grid': {'type': 'object', 'properties': {'columns': {'type': 'array'}, 'data': {'type': 'array'}, 'loading': {'type': 'boolean'}}, 'required': ['columns', 'data']},
    'table': {'type': 'object', 'properties': {'headers': {'type': 'array'}, 'rows': {'type': 'array'}}, 'required': ['headers', 'rows']},
    'chart': {'type': 'object', 'properties': {'type': {'type': 'string', 'enum': ['bar', 'line']}, 'data': {'type': 'array'}, 'title': {'type': 'string'}}, 'required': ['data']},
    'timeline': {'type': 'object', 'properties': {'events': {'type': 'array'}}, 'required': ['events']},
    'kanban': {'type': 'object', 'properties': {'columns': {'type': 'array'}}, 'required': ['columns']},
    'calendar': {'type': 'object', 'properties': {'year': {'type': 'number'}, 'month': {'type': 'number'}}, 'required': ['year', 'month']},
    'metric': {'type': 'object', 'properties': {'label': {'type': 'string'}, 'value': {}, 'unit': {'type': 'string'}, 'trend': {'type': 'string', 'enum': ['up', 'down', 'flat']}}, 'required': ['label', 'value']},
    'badge': {'type': 'object', 'properties': {'label': {'type': 'string'}, 'variant': {'type': 'string', 'enum': ['default', 'success', 'warning', 'error', 'info']}}, 'required': ['label']},

    // Input
    'form': {'type': 'object', 'properties': {'fields': {'type': 'array'}, 'submitLabel': {'type': 'string'}}, 'required': ['fields']},
    'text-field': {'type': 'object', 'properties': {'label': {'type': 'string'}, 'name': {'type': 'string'}, 'value': {'type': 'string'}, 'placeholder': {'type': 'string'}, 'required': {'type': 'boolean'}}, 'required': ['label', 'name']},
    'select': {'type': 'object', 'properties': {'label': {'type': 'string'}, 'name': {'type': 'string'}, 'options': {'type': 'array'}, 'value': {'type': 'string'}}, 'required': ['label', 'name', 'options']},
    'date-picker': {'type': 'object', 'properties': {'label': {'type': 'string'}, 'name': {'type': 'string'}, 'value': {'type': 'string'}}, 'required': ['label', 'name']},
    'search': {'type': 'object', 'properties': {'value': {'type': 'string'}, 'placeholder': {'type': 'string'}}},
    'file-upload': {'type': 'object', 'properties': {'label': {'type': 'string'}, 'name': {'type': 'string'}, 'accept': {'type': 'string'}, 'multiple': {'type': 'boolean'}}, 'required': ['label', 'name']},
    'json-editor': {'type': 'object', 'properties': {'label': {'type': 'string'}, 'name': {'type': 'string'}, 'value': {'type': 'string'}}, 'required': ['label', 'name']},
    'rich-editor': {'type': 'object', 'properties': {'label': {'type': 'string'}, 'name': {'type': 'string'}, 'value': {'type': 'string'}}, 'required': ['label', 'name']},

    // Action
    'button': {'type': 'object', 'properties': {'label': {'type': 'string'}, 'variant': {'type': 'string', 'enum': ['primary', 'secondary', 'ghost', 'destructive']}, 'size': {'type': 'string', 'enum': ['sm', 'md', 'lg']}, 'loading': {'type': 'boolean'}, 'disabled': {'type': 'boolean'}}, 'required': ['label']},
    'confirm': {'type': 'object', 'properties': {'message': {'type': 'string'}, 'confirmLabel': {'type': 'string'}, 'cancelLabel': {'type': 'string'}}, 'required': ['message']},
    'wizard': {'type': 'object', 'properties': {'steps': {'type': 'array'}}, 'required': ['steps']},
    'bulk-action': {'type': 'object', 'properties': {'selectedCount': {'type': 'number'}, 'actions': {'type': 'array'}}, 'required': ['selectedCount', 'actions']},
    'action-bar': {'type': 'object', 'properties': {'actions': {'type': 'array'}}, 'required': ['actions']},

    // Agent
    'agent-chat': {'type': 'object', 'properties': {'messages': {'type': 'array'}, 'loading': {'type': 'boolean'}}, 'required': ['messages']},
    'tool-call': {'type': 'object', 'properties': {'name': {'type': 'string'}, 'status': {'type': 'string', 'enum': ['pending', 'running', 'complete', 'error']}, 'args': {'type': 'string'}, 'result': {'type': 'string'}}, 'required': ['name', 'status']},
    'streaming-text': {'type': 'object', 'properties': {'text': {'type': 'string'}, 'streaming': {'type': 'boolean'}}, 'required': ['text']},
    'decision': {'type': 'object', 'properties': {'question': {'type': 'string'}, 'options': {'type': 'array'}}, 'required': ['question', 'options']},
    'progress-log': {'type': 'object', 'properties': {'entries': {'type': 'array'}, 'title': {'type': 'string'}}, 'required': ['entries']},
    'artifact': {'type': 'object', 'properties': {'type': {'type': 'string', 'enum': ['code', 'text', 'image', 'file']}, 'content': {'type': 'string'}, 'language': {'type': 'string'}, 'filename': {'type': 'string'}}, 'required': ['type', 'content']},

    // Navigation
    'nav-menu': {'type': 'object', 'properties': {'items': {'type': 'array'}, 'orientation': {'type': 'string', 'enum': ['horizontal', 'vertical']}}, 'required': ['items']},
    'command-palette': {'type': 'object', 'properties': {'open': {'type': 'boolean'}, 'commands': {'type': 'array'}}, 'required': ['open', 'commands']},
    'filter-bar': {'type': 'object', 'properties': {'filters': {'type': 'array'}, 'values': {'type': 'object'}}, 'required': ['filters', 'values']},
    'breadcrumb': {'type': 'object', 'properties': {'items': {'type': 'array'}}, 'required': ['items']},
  };
}
