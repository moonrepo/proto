{% macro cmd(args) %}
proto.exe run {{ bin }} {% if alt_bin %} --alt "{{ alt_bin }}" {% endif %}-- {{ before_args }} {{ args }} {{ after_args }}
{% endmacro input %}
