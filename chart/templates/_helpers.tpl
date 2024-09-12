{{/*
Expand the name of the chart.
*/}}
{{- define "app.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "app.fullname" -}}
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
{{- define "app.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "app.labels" -}}
helm.sh/chart: {{ include "app.chart" . }}
{{ include "app.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "app.selectorLabels" -}}
app.kubernetes.io/name: {{ include "app.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Deployment annotations
*/}}
{{- define "app.deployment.annotations" -}}
  {{- if .Values.deployment.annotations }}
  annotations:
    {{- $tp := typeOf .Values.deployment.annotations }}
    {{- if eq $tp "string" }}
      {{- tpl .Values.deployment.annotations . | nindent 4 }}
    {{- else }}
      {{- toYaml .Values.deployment.annotations | nindent 4 }}
    {{- end }}
  {{- end }}
{{- end -}}

{{/*
Pod annotations
*/}}
{{- define "app.pod.annotations" -}}
  {{- if .Values.deployment.podAnnotations }}
      annotations:
        {{- $tp := typeOf .Values.deployment.podAnnotations }}
        {{- if eq $tp "string" }}
          {{- tpl .Values.deployment.podAnnotations . | nindent 8 }}
        {{- else }}
          {{- toYaml .Values.deployment.podAnnotations | nindent 8 }}
        {{- end }}
  {{- end }}
{{- end -}}

{{/*
Affinity
*/}}
{{- define "app.affinity" -}}
  {{- if .Values.deployment.affinity }}
      affinity:
        {{ $tp := typeOf .Values.deployment.affinity }}
        {{- if eq $tp "string" }}
          {{- tpl .Values.deployment.affinity . | nindent 8 | trim }}
        {{- else }}
          {{- toYaml .Values.deployment.affinity | nindent 8 }}
        {{- end }}
  {{ end }}
{{- end -}}

{{/*
Node selector
*/}}
{{- define "app.nodeselector" -}}
  {{- if .Values.deployment.nodeSelector }}
      nodeSelector:
      {{- $tp := typeOf .Values.deployment.nodeSelector }}
      {{- if eq $tp "string" }}
        {{ tpl .Values.deployment.nodeSelector . | nindent 8 | trim }}
      {{- else }}
        {{- toYaml .Values.deployment.nodeSelector | nindent 8 }}
      {{- end }}
  {{- end }}
{{- end -}}

{{/*
Tolerations
*/}}
{{- define "app.tolerations" -}}
  {{- if .Values.deployment.tolerations }}
      tolerations:
      {{- $tp := typeOf .Values.deployment.tolerations }}
      {{- if eq $tp "string" }}
        {{ tpl .Values.deployment.tolerations . | nindent 8 | trim }}
      {{- else }}
        {{- toYaml .Values.deployment.tolerations | nindent 8 }}
      {{- end }}
  {{- end }}
{{- end -}}

{{/*
Security context for the deployment pod template
*/}}
{{- define "app.securityContext.pod" -}}
  {{- if .Values.deployment.securityContext.pod }}
      securityContext:
        {{- $tp := typeOf .Values.deployment.securityContext.pod }}
        {{- if eq $tp "string" }}
          {{- tpl .Values.deployment.securityContext.pod . | nindent 8 }}
        {{- else }}
          {{- toYaml .Values.deployment.securityContext.pod | nindent 8 }}
        {{- end }}
  {{- end }}
{{- end -}}

{{/*
Volumes
*/}}
{{- define "app.volumes" -}}
  {{- if .Values.deployment.volumes }}
      volumes:
      {{- $tp := typeOf .Values.deployment.volumes }}
      {{- if eq $tp "string" }}
        {{ tpl .Values.deployment.volumes . | nindent 8 | trim }}
      {{- else }}
        {{- toYaml .Values.deployment.volumes | nindent 8 }}
      {{- end }}
  {{- end }}
{{- end -}}

{{/*
Image Pull Secrets
*/}}
{{- define "imagePullSecrets" -}}
{{- with .Values.deployment.imagePullSecrets -}}
imagePullSecrets:
{{- range . -}}
{{- if typeIs "string" . }}
  - name: {{ . }}
{{- else if index . "name" }}
  - name: {{ .name }}
{{- end }}
{{- end -}}
{{- end -}}
{{- end -}}


{{/* 
Update strategy
*/}}
{{- define "app.deployment.strategy" -}}
  {{- if eq .Values.deployment.strategy.type "Recreate" }}
  strategy:
    type: Recreate
  {{- else if eq .Values.deployment.strategy.type "RollingUpdate" }}
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: {{ .Values.deployment.strategy.maxSurge | default "25%" }}
      maxUnavailable: {{ .Values.deployment.strategy.maxUnavailable | default "25%" }}
  {{- end }}
{{- end }}

{{/*
Init containers
*/}}
{{- define "app.initcontainers" -}}
  {{- if .Values.deployment.extraInitContainers }}
      initContainers:
      {{- $tp := typeOf .Values.deployment.extraInitContainers }}
      {{- if eq $tp "string" }}
        {{ tpl .Values.deployment.extraInitContainers . | nindent 8 | trim }}
      {{- else }}
        {{- toYaml .Values.deployment.extraInitContainers | nindent 8 }}
      {{- end }}
  {{- end }}
{{- end -}}

{{/*
Security context for the deployment app container
*/}}
{{- define "app.securityContext.container" -}}
  {{- if .Values.deployment.securityContext.container }}
          securityContext:
            {{- $tp := typeOf .Values.deployment.securityContext.container }}
            {{- if eq $tp "string" }}
              {{- tpl .Values.deployment.securityContext.container . | nindent 12 }}
            {{- else }}
              {{- toYaml .Values.deployment.securityContext.container | nindent 12 }}
            {{- end }}
  {{- end }}
{{- end -}}

{{/*
Environment variables
*/}}
{{- define "app.env" -}}
  {{- if .Values.deployment.env }}
          env:
      {{- $tp := typeOf .Values.deployment.env }}
      {{- if eq $tp "string" }}
      {{ tpl .Values.deployment.env . | nindent 12 | trim }}
      {{- else }}
      {{- toYaml .Values.deployment.env | nindent 12 }}
      {{- end }}
  {{- end }}
{{- end -}}

{{/*
Volume Mounts
*/}}
{{- define "app.mounts" -}}
  {{- if .Values.deployment.volumeMounts }}
          volumeMounts:
      {{- $tp := typeOf .Values.deployment.volumeMounts }}
      {{- if eq $tp "string" }}
        {{ tpl .Values.deployment.volumeMounts . | nindent 10 | trim }}
      {{- else }}
        {{- toYaml .Values.deployment.volumeMounts | nindent 10 }}
      {{- end }}
  {{- end }}
{{- end -}}

{{/*
Container resources
*/}}
{{- define "app.resources" -}}
  {{- if .Values.deployment.resources -}}
          resources:
{{ toYaml .Values.deployment.resources | indent 12}}
  {{ end }}
{{- end -}}

{{/*
Service annotations
*/}}
{{- define "app.service.annotations" -}}
  {{- if .Values.service.annotations }}
  annotations:
    {{- $tp := typeOf .Values.service.annotations }}
    {{- if eq $tp "string" }}
      {{- tpl .Values.service.annotations . | nindent 4 }}
    {{- else }}
      {{- toYaml .Values.service.annotations | nindent 4 }}
    {{- end }}
  {{- end }}
{{- end -}}

{{/*
Ingress annotations
*/}}
{{- define "app.ingress.annotations" -}}
  {{- if .Values.ingress.annotations }}
  annotations:
    {{- $tp := typeOf .Values.ingress.annotations }}
    {{- if eq $tp "string" }}
      {{- tpl .Values.ingress.annotations . | nindent 4 }}
    {{- else }}
      {{- toYaml .Values.ingress.annotations | nindent 4 }}
    {{- end }}
  {{- end }}
{{- end -}}

{{/*
Create the name of the service account to use
*/}}
{{- define "app.serviceAccount.name" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "app.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Service account annotations
*/}}
{{- define "app.serviceAccount.annotations" -}}
  {{- if .Values.serviceAccount.annotations }}
  annotations:
    {{- $tp := typeOf .Values.serviceAccount.annotations }}
    {{- if eq $tp "string" }}
      {{- tpl .Values.serviceAccount.annotations . | nindent 4 }}
    {{- else }}
      {{- toYaml .Values.serviceAccount.annotations | nindent 4 }}
    {{- end }}
  {{- end }}
{{- end -}}
