{{/*
Expand the name of the chart.
*/}}
{{- define "flint-forge.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "flint-forge.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "flint-forge.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "flint-forge.labels" -}}
helm.sh/chart: {{ include "flint-forge.chart" . }}
{{ include "flint-forge.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "flint-forge.selectorLabels" -}}
app.kubernetes.io/name: {{ include "flint-forge.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Database URL for app containers.
*/}}
{{- define "flint-forge.databaseUrl" -}}
{{- if .Values.postgres.enabled }}
{{- printf "postgres://%s:%s@%s-postgres:5432/%s" .Values.postgres.user .Values.postgres.password (include "flint-forge.fullname" .) .Values.postgres.database }}
{{- else }}
{{- required "DATABASE_URL is required when postgres.enabled=false" .Values.externalDatabaseUrl }}
{{- end }}
{{- end }}
