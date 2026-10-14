from alembic import op

revision = "0001"
down_revision = None


def upgrade():
    op.create_table("t")


def downgrade():
    op.drop_table("t")
